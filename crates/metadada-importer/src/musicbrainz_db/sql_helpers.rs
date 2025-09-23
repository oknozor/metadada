use crate::MbLight;
use std::io::Read;

use anyhow::Context;
use anyhow::Result;
use bytes::Bytes;
use indicatif::ProgressBar;
use sqlx::postgres::PgPoolCopyExt;
use std::fs;
use tar::Entry;
use tracing::info;

impl MbLight {
    pub async fn pg_copy(
        &self,
        mut entry: Entry<'_, impl Read>,
        schema: &str,
        table: &str,
        pb: ProgressBar,
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

            pb.inc(n as u64);
        }

        sink.finish().await.context("Failed to close sink")?;

        pb.set_message(format!("Committing on {schema}.{table}"));
        tx.commit().await.context("Failed to commit transaction")?;
        sqlx::query(&format!("ALTER TABLE {}.{} SET LOGGED", schema, table))
            .execute(&self.db)
            .await
            .context("Failed to restore LOGGED on table")?;

        pb.finish_with_message(format!("{schema}.{table} COPY done!"));
        Ok(())
    }

    pub async fn run_sql_file(&self, path: &str) -> Result<()> {
        info!("Executing SQL file: {}", path);
        let sql = fs::read_to_string(path)?;
        let sql = sql
            .lines()
            .filter(|line| !line.trim_start().starts_with('\\'))
            .collect::<Vec<_>>()
            .join("\n");
        sqlx::query("SET search_path TO musicbrainz, public")
            .execute(&self.db)
            .await?;
        sqlx::raw_sql(&sql).execute(&self.db).await?;

        Ok(())
    }
}
