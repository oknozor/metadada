use std::io::Read;
use std::path::Path;

use crate::download::github;
use crate::{MbLight, download::musicbrainz::MUSICBRAINZ_FTP, tar_helper::get_archive};
use anyhow::Context;
use anyhow::Result;
use bytes::Bytes;
use sqlx::postgres::PgPoolCopyExt;
use std::{fs, path::PathBuf};
use tar::Entry;
use tempfile::NamedTempFile;
use tracing::{error, info};

const MB_DUMP: &str = "mbdump.tar.bz2";
const MB_DUMP_DERIVED: &str = "mbdump-derived.tar.bz2";
const COVER_ART_ARCHIVE: &str = "mbdump-cover-art-archive.tar.bz2";
const EVENT_ART_ARCHIVE: &str = "mbdump-even-art-archive.tar.bz2";
const MB_DUMP_STATS: &str = "mbdump-stats.tar.bz2";

impl MbLight {
    pub async fn init(&mut self) -> Result<()> {
        let local_path = github::download_musicbrainz_sql().await?;
        self.create_schemas().await?;
        self.create_tables(&local_path).await?;
        self.ingest_musicbrainz_data().await?;
        self.run_all_scripts(local_path).await?;
        Ok(())
    }

    async fn create_schemas(&self) -> Result<()> {
        let schemas = [
            "musicbrainz",
            "cover_art_archive",
            "event_art_archive",
            "statistics",
            "documentation",
            "wikidocs",
            "dbmirror2",
        ];

        for schema in schemas {
            if self.config.schema.should_skip(schema) {
                continue;
            }

            let query = format!("CREATE SCHEMA IF NOT EXISTS {}", schema);
            info!("Executing query: {}", query);
            sqlx::query(&query).execute(&self.db).await?;
        }

        Ok(())
    }

    async fn run_all_scripts(&mut self, local_path: PathBuf) -> Result<()> {
        let sql_scripts = vec![
            ("musicbrainz", "CreatePrimaryKeys.sql"),
            ("cover_art_archive", "caa/CreatePrimaryKeys.sql"),
            ("event_art_archive", "eaa/CreatePrimaryKeys.sql"),
            ("statistics", "statistics/CreatePrimaryKeys.sql"),
            ("documentation", "documentation/CreatePrimaryKeys.sql"),
            ("wikidocs", "wikidocs/CreatePrimaryKeys.sql"),
            ("musicbrainz", "CreateFunctions.sql"),
            ("musicbrainz", "CreateMirrorOnlyFunctions.sql"),
            ("cover_art_archive", "caa/CreateFunctions.sql"),
            ("event_art_archive", "eaa/CreateFunctions.sql"),
            ("musicbrainz", "CreateIndexes.sql"),
            ("musicbrainz", "CreateMirrorIndexes.sql"),
            ("cover_art_archive", "caa/CreateIndexes.sql"),
            ("event_art_archive", "eaa/CreateIndexes.sql"),
            ("statistics", "statistics/CreateIndexes.sql"),
            ("musicbrainz", "CreateViews.sql"),
            ("cover_art_archive", "caa/CreateViews.sql"),
            ("event_art_archive", "eaa/CreateViews.sql"),
            ("musicbrainz", "CreateMirrorOnlyTriggers.sql"),
            ("musicbrainz", "ReplicationSetup.sql"),
            ("dbmirror2", "dbmirror2/ReplicationSetup.sql"),
        ];

        for (schema, sql_script) in sql_scripts {
            if self.config.schema.should_skip(schema) {
                continue;
            }
            let path = local_path.join(sql_script);
            self.run_sql_file(path.to_str().unwrap()).await?;
        }

        Ok(())
    }

    async fn create_tables(&mut self, local_path: &Path) -> Result<()> {
        self.run_sql_file(local_path.join("Extensions.sql").to_str().unwrap())
            .await?;
        self.run_sql_file(
            local_path
                .join("CreateSearchConfiguration.sql")
                .to_str()
                .unwrap(),
        )
        .await?;
        let sql_scripts = vec![
            // types
            ("musicbrainz", "CreateCollations.sql"),
            ("musicbrainz", "CreateTypes.sql"),
            // tables
            ("musicbrainz", "CreateTables.sql"),
            ("cover_art_archive", "caa/CreateTables.sql"),
            ("event_art_archive", "eaa/CreateTables.sql"),
            ("statistics", "statistics/CreateTables.sql"),
            ("documentation", "documentation/CreateTables.sql"),
            ("wikidocs", "wikidocs/CreateTables.sql"),
        ];
        for (schema, sql_script) in sql_scripts {
            if self.config.schema.should_skip(schema) {
                continue;
            }
            let path = local_path.join(sql_script);
            self.run_sql_file(path.to_str().unwrap()).await?;
        }
        Ok(())
    }
}

impl MbLight {
    async fn ingest_musicbrainz_data(&mut self) -> Result<()> {
        let mut filenames = vec![MB_DUMP, MB_DUMP_DERIVED];

        if !self.config.schema.should_skip("statistics") {
            filenames.push(MB_DUMP_STATS);
        }
        if !self.config.schema.should_skip("cover_art_archive") {
            filenames.push(COVER_ART_ARCHIVE);
        }
        if !self.config.schema.should_skip("event_art_archive") {
            filenames.push(EVENT_ART_ARCHIVE);
        }

        let latest = self.get_latest().await?;
        info!("Latest version: {}", latest);

        for filename in filenames {
            let url = format!("{}/{}/{}", MUSICBRAINZ_FTP, latest, filename);
            let tempfile = NamedTempFile::new()?;
            let mut writer = tempfile.reopen()?;
            self.download_with_progress(&url, &mut writer).await?;
            let mut archive = get_archive(tempfile.path())?;

            info!("Starting pg_copy for {filename}");

            for entry in archive.entries()? {
                match entry {
                    Ok(entry) => {
                        let path = entry.path()?;
                        #[cfg(feature = "progress")]
                        let entry_size = entry.header().entry_size()?;
                        let name = path.to_string_lossy().into_owned();

                        if !name.starts_with("mbdump/") {
                            continue;
                        }

                        let filename = name.strip_prefix("mbdump/").unwrap();
                        let filename = filename.strip_suffix("_sanitised").unwrap_or(filename);

                        let (schema, table) = filename
                            .split_once('.')
                            .unwrap_or(("musicbrainz", filename));

                        if self.should_skip_table(schema, table).await? {
                            continue;
                        }

                        #[cfg(feature = "progress")]
                        let pb = {
                            let pb = ProgressBar::new(entry_size);
                            let style = ProgressStyle::default_bar()
                                            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta}) - {msg}")
                                            .unwrap()
                                            .progress_chars("#>-");
                            pb.set_style(style);
                            pb.set_message(table.to_string());
                        };

                        self.pg_copy(entry, schema, table)
                            .await
                            .context(format!("in {schema}.{table}"))?;
                    }
                    Err(err) => {
                        error!("{err}");
                        break;
                    }
                }
            }
        }

        Ok(())
    }

    async fn pg_copy(
        &self,
        mut entry: Entry<'_, impl Read>,
        schema: &str,
        table: &str,
        #[cfg(feature = "progress")] pb: ProgressBar,
    ) -> Result<(), anyhow::Error> {
        sqlx::query(&format!("ALTER TABLE {}.{} SET UNLOGGED", schema, table))
            .execute(&self.db)
            .await
            .context("Failed to set table to UNLOGGED")?;

        let tx = self.db.begin().await?;

        let mut sink = self
            .db
            .copy_in_raw(&format!("COPY {}.{} FROM STDIN", schema, table))
            .await
            .context("Failed to start COPY")?;

        let mut buffer = vec![0u8; 8 * 1024 * 1024];

        loop {
            let n = entry
                .read(&mut buffer)
                .context("Failed to read from archive entry")?;
            if n == 0 {
                break;
            }

            let chunk = Bytes::copy_from_slice(&buffer[..n]);
            sink.send(chunk)
                .await
                .context("Failed to send data chunk to Postgres")?;

            #[cfg(feature = "progress")]
            pb.inc(n as u64);
        }

        sink.finish().await.context("Failed to close sink")?;

        #[cfg(feature = "progress")]
        pb.set_message(format!("Committing on {schema}.{table}"));
        tx.commit().await.context("Failed to commit transaction")?;
        sqlx::query(&format!("ALTER TABLE {}.{} SET LOGGED", schema, table))
            .execute(&self.db)
            .await
            .context("Failed to restore LOGGED on table")?;

        #[cfg(feature = "progress")]
        pb.finish_with_message(format!("{schema}.{table} COPY done!"));
        Ok(())
    }

    async fn run_sql_file(&self, path: &str) -> Result<()> {
        info!("Executing SQL file: {}", path);
        let sql = fs::read_to_string(path)?;
        let sql = sql
            .lines()
            .filter(|line| !line.trim_start().starts_with('\\'))
            .collect::<Vec<_>>()
            .join("\n");

        sqlx::raw_sql(&sql).execute(&self.db).await?;
        sqlx::query("SET search_path TO musicbrainz, public")
            .execute(&self.db)
            .await?;

        Ok(())
    }
}
