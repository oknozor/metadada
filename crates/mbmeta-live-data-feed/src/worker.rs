use std::{
    fs::File,
    io::{BufReader, Read},
    path::Path,
};

use crate::{dbmirror, download::ReplicationPacketFetcher, replication_packet::ReplicationControl};
use anyhow::Result;
use bzip2::bufread::BzDecoder;
use chrono::{DateTime, Utc};
use mbmeta_settings::Settings;
use sqlx::PgPool;
use tar::Archive;
use tempfile::NamedTempFile;
use tokio::time;
use tracing::{error, info};

pub struct MusicbrainzReplicationWorker {
    db: PgPool,
    fetcher: ReplicationPacketFetcher,
}

impl MusicbrainzReplicationWorker {
    pub fn new(db: PgPool, config: &Settings) -> Self {
        Self {
            db,
            fetcher: ReplicationPacketFetcher::new(
                config.musicbrainz.url.clone(),
                config.musicbrainz.token.clone(),
            ),
        }
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        loop {
            let replication_control = ReplicationControl::get(&self.db).await?;
            if let Some(next_replication_sequence) = replication_control.next_replication_sequence()
            {
                let last_replication_date = replication_control
                    .last_replication_date
                    .map(|d| d.format("%y/%m/%d - %H:%M:%S").to_string())
                    .unwrap_or("N/a".into());

                info!(
                    "Starting new replication process, last replication occured on {last_replication_date}",
                );

                let mut tmpfile = NamedTempFile::new()?;
                sqlx::query!("TRUNCATE dbmirror2.pending_data, dbmirror2.pending_keys")
                    .execute(&self.db)
                    .await?;
                let writer = tmpfile.as_file_mut();
                self.fetcher
                    .fetch_packet(next_replication_sequence, writer)
                    .await?;

                dbmirror::truncate_tables(&self.db).await?;
                info!(
                    "Replication packet {} downloaded, processing...",
                    next_replication_sequence
                );
                let mut archive = get_archive(tmpfile.path())?;
                for entry in archive.entries()? {
                    match entry {
                        Ok(mut entry) => {
                            let path = entry.path()?;

                            let filename = path.as_ref().file_name().and_then(|f| f.to_str());

                            match filename {
                                Some("pending_keys") => {
                                    dbmirror::load_pending_keys(&self.db, &mut entry).await?;
                                }
                                Some("pending_data") => {
                                    dbmirror::load_pending_data(&self.db, &mut entry).await?;
                                }
                                Some("REPLICATION_SEQUENCE") => {
                                    let mut replication_sequence = String::new();
                                    let _ = entry.read_to_string(&mut replication_sequence);
                                    let replication_sequence = replication_sequence.trim();
                                    let replication_sequence =
                                        replication_sequence.parse::<i32>()?;

                                    if replication_sequence != next_replication_sequence {
                                        error!(
                                            "Replication sequence mismatch: expected {}, got {}",
                                            next_replication_sequence, replication_sequence
                                        );
                                        time::sleep(std::time::Duration::from_secs(30 * 60)).await;
                                        continue;
                                    }
                                }
                                Some("SCHEMA_SEQUENCE") => {
                                    let mut schema_sequence = String::new();
                                    let _ = entry.read_to_string(&mut schema_sequence)?;
                                    let schema_sequence = schema_sequence.trim();
                                    let schema_sequence = schema_sequence.parse::<i32>()?;
                                    if !replication_control.schema_sequence_match(schema_sequence) {
                                        error!(
                                            "Schema sequence mismatch: expected {}, got {}",
                                            replication_control
                                                .current_schema_sequence
                                                .unwrap_or_default(),
                                            schema_sequence
                                        );
                                        time::sleep(std::time::Duration::from_secs(30 * 60)).await;
                                        continue;
                                    }
                                }
                                Some("TIMESTAMP") => {
                                    extract_timestamp(entry)?;
                                }
                                _ => {}
                            }
                        }
                        Err(err) => {
                            error!("Error skipping archive entry: {err}");
                            break;
                        }
                    }
                }
                dbmirror::replicate(&self.db).await?;

                info!("replication finished, sleeping");
                tokio::time::sleep(std::time::Duration::from_secs(30 * 60)).await;
            }
        }
    }
}

fn extract_timestamp(mut entry: impl std::io::Read) -> anyhow::Result<()> {
    let mut date_str = String::new();
    entry.read_to_string(&mut date_str)?;
    let date_str = date_str.trim();
    let date_str = if date_str.ends_with("+00") || date_str.ends_with("-00") {
        format!("{}:00", date_str)
    } else {
        date_str.to_string()
    };

    let date = DateTime::parse_from_str(&date_str, "%Y-%m-%d %H:%M:%S%.f%:z")?.with_timezone(&Utc);
    let date = date.format("%Y-%m-%d %H:%M:%S");
    info!("replication packet emitted at: {date}");
    Ok(())
}

fn get_archive(tmpfile: &Path) -> Result<Archive<impl Read>> {
    let f = File::open(tmpfile)?;
    let reader = BufReader::new(f);
    let decompressor = BzDecoder::new(reader);
    let archive = Archive::new(decompressor);
    Ok(archive)
}
