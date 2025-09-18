use crate::{dbmirror, download::ReplicationPacketFetcher, replication_packet::ReplicationControl};
use anyhow::Result;
use async_compression::tokio::bufread::BzDecoder;
use async_tar::Archive;
use chrono::{DateTime, Utc};
use futures_util::{AsyncReadExt, StreamExt};
use sqlx::PgPool;
use tempfile::NamedTempFile;
use tokio::{fs::File, io::BufReader, time};
use tokio_util::compat::TokioAsyncReadCompatExt;
use tracing::{error, info, warn};

type TarEntry = tokio_util::compat::Compat<
    async_compression::tokio::bufread::BzDecoder<tokio::io::BufReader<tokio::fs::File>>,
>;

pub struct MusicbrainzReplicationWorker {
    db: PgPool,
    fetcher: ReplicationPacketFetcher,
}

impl MusicbrainzReplicationWorker {
    pub async fn new(db: PgPool) -> Self {
        Self {
            db,
            fetcher: ReplicationPacketFetcher::new(),
        }
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        loop {
            let replication_control = ReplicationControl::get(&self.db).await?;
            if let Some(next_replication_sequence) = replication_control.next_replication_sequence()
            {
                let tmpfile = NamedTempFile::new()?;
                let mut writer = tokio::fs::File::from_std(tmpfile.reopen()?);
                match self
                    .fetcher
                    .fetch_packet(next_replication_sequence, &mut writer)
                    .await
                {
                    Ok(_) => {
                        info!(
                            "Replication packet {} downloaded, processing...",
                            next_replication_sequence
                        );
                        let archive = get_archive(tmpfile).await?;
                        let mut entries = archive.entries()?;
                        while let Some(entry) = entries.next().await {
                            let mut entry = entry?;
                            let path = entry.path()?;
                            let filename = path.as_ref().file_name().and_then(|f| f.to_str());

                            match filename {
                                Some("dbmirror_pending") => {
                                    dbmirror::load_pending_keys(&self.db, &mut entry).await?;
                                }
                                Some("dbmirror_pending_data") => {
                                    dbmirror::load_pending_data(&self.db, &mut entry).await?;
                                }
                                Some("REPLICATION_SEQUENCE") => {
                                    let mut replication_sequence = String::new();
                                    let _ = entry.read_to_string(&mut replication_sequence);
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
                                    let _ = entry.read_to_string(&mut schema_sequence).await?;
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
                                    extract_timestamp(entry).await?;
                                }
                                _ => {}
                            }
                        }
                    }
                    Err(e) => {
                        warn!("{}", e);
                        info!("sleeping for 30m");
                        time::sleep(std::time::Duration::from_secs(30 * 60)).await;
                        continue;
                    }
                }
            }
        }
    }
}

async fn extract_timestamp(mut entry: impl AsyncReadExt + Unpin) -> anyhow::Result<()> {
    let mut timestamp = String::new();
    let _ = entry.read_to_string(&mut timestamp).await?;
    let date = DateTime::parse_from_str(&timestamp, "%Y-%m-%d %H:%M:%S%.f%:z")?.with_timezone(&Utc);
    let date = date.format("%Y-%m-%d %H:%M:%S");
    info!("replication packet emitted at: {date}");
    Ok(())
}

async fn get_archive(tmpfile: NamedTempFile) -> Result<Archive<TarEntry>> {
    let f = File::open(tmpfile.path()).await?;
    let reader = BufReader::new(f);
    let decompressor = BzDecoder::new(reader);
    let compat_reader = decompressor.compat();
    let archive = Archive::new(compat_reader);
    Ok(archive)
}
