use std::{collections::BTreeMap, io::Read};

use crate::{
    MbLight,
    error::ReplicationError,
    musicbrainz_db::replication::{
        pending_data::{PendindingKey, PendingData, sort_pending_data},
        replication_control::ReplicationControl,
    },
    tar_helper::get_archive,
};
use anyhow::anyhow;
use sqlx::{
    PgPool,
    types::chrono::{DateTime, Utc},
};
use tempfile::NamedTempFile;
use tokio::time;
use tracing::{error, info};

mod pending_data;
mod replication_control;

impl MbLight {
    pub async fn apply_all_pending_replication(&self) -> Result<(), ReplicationError> {
        loop {
            match self.apply_pending_replication().await {
                Ok(_) => {}
                Err(ReplicationError::NotFound) => {
                    info!("Reached last replication packet, sending reindex signal");
                    self.reindex_sender.send(()).await?;
                    info!("Waiting for 15 minutes for a fresh replication packet");
                    time::sleep(time::Duration::from_secs(60 * 15)).await;
                }
                Err(err) => {
                    error!("Fatal error applying pending replication: {}", err);
                    return Err(err);
                }
            }
        }
    }

    pub async fn save_next_sequence(&self, seq: i32, db: &PgPool) -> sqlx::Result<()> {
        sqlx::query!(
            r#"
            UPDATE replication_control
            SET
                current_replication_sequence = $1,
                last_replication_date = NOW()
            "#,
            seq
        )
        .execute(db)
        .await?;

        Ok(())
    }

    pub async fn apply_pending_replication(&self) -> Result<(), ReplicationError> {
        let replication_control = ReplicationControl::get(&self.db).await?;
        if let Some(next_replication_sequence) = replication_control.next_replication_sequence() {
            let last_replication_date = replication_control
                .last_replication_date
                .map(|d| d.format("%y/%m/%d - %H:%M:%S").to_string())
                .unwrap_or("N/a".into());
            info!(
                "Starting new replication process, last replication occured on {last_replication_date}",
            );
            let tmpfile = NamedTempFile::new()?;
            {
                let mut writer = tmpfile.reopen()?;
                let packet_url = replication_control
                    .next_replication_packet_url(
                        &self.config.musicbrainz.url,
                        &self.config.musicbrainz.token,
                    )
                    .ok_or(anyhow!("Failed to get next replication packet URL"))?;

                self.download_with_progress(&packet_url, &mut writer)
                    .await?;
            }
            info!(
                "Replication packet {} downloaded, processing...",
                next_replication_sequence
            );
            let mut archive = get_archive(tmpfile.path())?;
            let mut pending_data = vec![];
            let mut pending_keys = BTreeMap::new();
            for entry in archive.entries()? {
                match entry {
                    Ok(mut entry) => {
                        let path = entry.path()?;
                        let filename = path.as_ref().file_name().and_then(|f| f.to_str());
                        info!("processing {}", filename.unwrap_or("unknown"));
                        match filename {
                            Some("pending_data") => {
                                let reader = csv::ReaderBuilder::new()
                                    .delimiter(b'\t')
                                    .has_headers(false)
                                    .from_reader(entry);

                                let mut reader = reader;

                                for result in reader.deserialize() {
                                    let record: PendingData = result?;
                                    pending_data.push(record);
                                }
                            }
                            Some("pending_keys") => {
                                let reader = csv::ReaderBuilder::new()
                                    .delimiter(b'\t')
                                    .has_headers(false)
                                    .from_reader(entry);

                                let mut reader = reader;

                                for result in reader.deserialize() {
                                    let record: PendindingKey = result?;
                                    let (fulltable, keys) = record.into_entry();
                                    pending_keys.insert(fulltable, keys);
                                }
                            }
                            Some("REPLICATION_SEQUENCE") => {
                                let mut replication_sequence = String::new();
                                let _ = entry.read_to_string(&mut replication_sequence);
                                let replication_sequence = replication_sequence.trim();
                                let replication_sequence = replication_sequence.parse::<i32>()?;
                                if replication_sequence != next_replication_sequence {
                                    tracing::error!(
                                        "Replication sequence mismatch: expected {}, got {}",
                                        next_replication_sequence,
                                        replication_sequence
                                    );
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
                    }
                }
            }

            pending_data.retain(|p| {
                let (schema, table) = p.split_table_schema();
                !self.config.schema.should_skip(schema) && !self.config.tables.should_skip(table)
            });

            let pending_data = sort_pending_data(pending_data);
            self.apply_replication(&pending_data, &pending_keys).await?;
            info!("replication finished, sleeping");
            replication_control.update(&self.db).await?;
        }

        Ok(())
    }

    async fn apply_replication(
        &self,
        pending_data: &[PendingData],
        pending_keys: &BTreeMap<String, Vec<String>>,
    ) -> anyhow::Result<()> {
        let mut tx = self.db.begin().await?;
        info!("processing {} pending data", pending_data.len());
        for data in pending_data {
            match data.to_sql_inline(pending_keys) {
                Ok(Some(query)) => {
                    sqlx::query(&query).execute(&mut *tx).await?;
                }
                Err(e) => {
                    error!("Failed to process pending data: {data:?}");
                    return Err(e)?;
                }
                Ok(None) => {}
            }
        }
        tx.commit().await?;
        Ok(())
    }
}

fn extract_timestamp(mut entry: impl std::io::Read) -> anyhow::Result<()> {
    let mut date_str = String::new();
    entry.read_to_string(&mut date_str)?;
    let date_str = date_str.trim();
    info!("raw timestamp: {:?}", date_str);

    // Append ":00" to make timezone compatible with %:z
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
