use std::io::Read;

use crate::{
    MbLight,
    error::ReplicationError,
    musicbrainz_db::replication::{
        pending_data::PendingData, replication_control::ReplicationControl,
    },
    progress::get_progress_bar,
    tar_helper::get_archive,
};
use anyhow::anyhow;
use itertools::Itertools;
use sqlx::types::chrono::{DateTime, Utc};
use tempfile::NamedTempFile;
use tokio::time;
use tracing::{debug, error, info};

mod pending_data;
mod replication_control;

impl MbLight {
    pub async fn apply_all_pending_replication(&self) -> Result<(), ReplicationError> {
        self.drop_tablecheck().await?;
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

    pub async fn apply_pending_replication(&self) -> Result<(), ReplicationError> {
        let remains = PendingData::all(&self.db).await?;
        if !remains.is_empty() {
            let replication_control = ReplicationControl::get(&self.db).await?;
            info!("Applying unfinished replication packet");
            self.apply_pending_data().await?;
            info!("Replication finished");
            replication_control.update(&self.db).await?;
        }

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

            for entry in archive.entries()? {
                self.process_replication_entry(
                    &replication_control,
                    entry,
                    next_replication_sequence,
                )
                .await?;
            }

            self.apply_pending_data().await?;
            info!("replication finished");
            replication_control.update(&self.db).await?;
        }

        Ok(())
    }

    async fn process_replication_entry(
        &self,
        replication_control: &ReplicationControl,
        entry: Result<tar::Entry<'_, impl Read>, std::io::Error>,
        next_replication_sequence: i32,
    ) -> Result<(), ReplicationError> {
        match entry {
            Ok(mut entry) => {
                let path = entry.path()?;
                let filename = path.as_ref().file_name().and_then(|f| f.to_str());
                debug!("processing {}", filename.unwrap_or("unknown"));
                match filename {
                    Some("pending_data") => {
                        let pb = get_progress_bar(entry.size())?;
                        self.pg_copy(entry, "dbmirror2", "pending_data", pb).await?;
                    }
                    Some("pending_keys") => {
                        let pb = get_progress_bar(entry.size())?;
                        self.pg_copy(entry, "dbmirror2", "pending_keys", pb).await?;
                    }
                    Some("REPLICATION_SEQUENCE") => {
                        let mut replication_sequence = String::new();
                        let _ = entry.read_to_string(&mut replication_sequence);
                        let replication_sequence = replication_sequence.trim();
                        let replication_sequence = replication_sequence.parse::<i32>()?;
                        if replication_sequence != next_replication_sequence {
                            return Err(ReplicationError::SequenceMissmatch {
                                expected: next_replication_sequence,
                                got: replication_sequence,
                            });
                        }
                    }
                    Some("SCHEMA_SEQUENCE") => {
                        let mut schema_sequence = String::new();
                        let _ = entry.read_to_string(&mut schema_sequence)?;
                        let schema_sequence = schema_sequence.trim();
                        let schema_sequence = schema_sequence.parse::<i32>()?;
                        if !replication_control.schema_sequence_match(schema_sequence) {
                            return Err(ReplicationError::SchemaMissmatch {
                                expected: replication_control
                                    .current_replication_sequence
                                    .unwrap_or_default(),
                                got: schema_sequence,
                            });
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
        };

        Ok(())
    }

    async fn apply_pending_data(&self) -> anyhow::Result<()> {
        let pending_data = PendingData::all(&self.db).await?;
        info!("Processing {} pending data ...", pending_data.len());
        let pb = get_progress_bar(pending_data.len() as u64)?;
        let chunked_data = pending_data.into_iter().chunk_by(|data| data.xid);

        for (xid, group) in chunked_data.into_iter() {
            let mut tx = self.db.begin().await?;
            for data in group {
                match data.to_sql_inline() {
                    Ok(Some(query)) => {
                        sqlx::query(&query).execute(&mut *tx).await?;
                    }
                    Err(e) => {
                        error!("Failed to process pending data: {data:?}");
                        pb.finish_and_clear();
                        return Err(e);
                    }
                    Ok(None) => {}
                }
                pb.inc(1);
            }
            pb.set_message("Committing ...");
            tx.commit().await?;
            pb.set_message(format!("Removing pending data for xid {}", xid));
            PendingData::remove_by_xid(&self.db, xid).await?;
        }
        self.truncate_pending_data().await?;
        pb.finish_with_message("Replication completed");
        Ok(())
    }

    async fn drop_tablecheck(&self) -> anyhow::Result<()> {
        sqlx::query!(
            "ALTER TABLE dbmirror2.pending_data DROP CONSTRAINT IF EXISTS tablename_exists;"
        )
        .execute(&self.db)
        .await?;
        Ok(())
    }
}

fn extract_timestamp(mut entry: impl std::io::Read) -> anyhow::Result<()> {
    let mut date_str = String::new();
    entry.read_to_string(&mut date_str)?;
    let date_str = date_str.trim();
    debug!("Raw timestamp: {:?}", date_str);

    // Append ":00" to make timezone compatible with %:z
    let date_str = if date_str.ends_with("+00") || date_str.ends_with("-00") {
        format!("{}:00", date_str)
    } else {
        date_str.to_string()
    };

    let date = DateTime::parse_from_str(&date_str, "%Y-%m-%d %H:%M:%S%.f%:z")?.with_timezone(&Utc);
    let date = date.format("%Y-%m-%d %H:%M:%S");
    info!("Replication packet emitted at: {date}");
    Ok(())
}
