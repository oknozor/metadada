use anyhow::Result;
use futures::stream::{self, StreamExt};
use indicatif::{ProgressBar, ProgressStyle};
use metadada_db::{Data, queryables::QueryAble};
use metadada_meili::{MeiliClient, Status};
use sqlx::{PgPool, types::Uuid};
use std::time::{Duration, Instant};
use tracing::{error, info};

/// Adaptive batch sizer that adjusts the batch size after each batch to keep
/// each batch's total duration close to `target_duration`. The size is clamped
/// between `min_size` and `max_size`.
pub struct AdaptiveBatchSizer {
    current: i64,
    min_size: i64,
    max_size: i64,
    target_duration: Duration,
}

impl AdaptiveBatchSizer {
    pub fn new(initial: i64, target_duration: Duration) -> Self {
        Self {
            current: initial,
            min_size: (initial / 4).max(1),
            max_size: initial * 4,
            target_duration,
        }
    }

    pub fn current(&self) -> i64 {
        self.current
    }

    /// Call after each batch with the time it took. Adjusts `current` size
    /// proportionally: new_size = current * (target / elapsed), clamped.
    pub fn adjust(&mut self, elapsed: Duration) {
        let elapsed_secs = elapsed.as_secs_f64().max(0.001);
        let target_secs = self.target_duration.as_secs_f64();
        let ratio = target_secs / elapsed_secs;
        let new_size =
            ((self.current as f64 * ratio).round() as i64).clamp(self.min_size, self.max_size);

        if new_size != self.current {
            info!(
                "Adaptive batch size: {} → {} (batch took {:.2}s, target {:.2}s)",
                self.current, new_size, elapsed_secs, target_secs,
            );
            self.current = new_size;
        }
    }
}

#[derive(Clone)]
pub struct Ingestor {
    pub db: PgPool,
    pub meili_client: MeiliClient,
}

impl Ingestor {
    pub async fn batch_ingest<T: QueryAble>(&self) -> Result<()> {
        let concurrency = 10;
        let last_seen_gid: Option<Uuid> = Some(Uuid::nil());
        let total: i64 = T::count(&self.db).await?;
        let pb = ProgressBar::new(total as u64);
        pb.set_style(
            ProgressStyle::with_template(
                "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})",
            )?
            .progress_chars("#>-"),
        );

        // batch_size() is the configured initial size; target 5 s per batch
        let sizer = AdaptiveBatchSizer::new(T::batch_size(), Duration::from_secs(5));

        // We drive the stream sequentially for size feedback, then fan-out
        // the ingest work with buffer_unordered.
        let stream = stream::unfold((last_seen_gid, sizer), |(last_gid, mut sizer)| async move {
            let this = self.clone();
            let t0 = Instant::now();

            match T::query_all(last_gid, sizer.current(), &this.db).await {
                Ok(Data { items }) => {
                    let items: Vec<T> = match items {
                        Some(a) if !a.is_empty() => a.0,
                        _ => return None,
                    };

                    sizer.adjust(t0.elapsed());
                    let next_gid = items.last().map(|a| a.id());
                    Some((Ok((this, items)), (next_gid, sizer)))
                }
                Err(e) => Some((Err(e), (last_gid, sizer))),
            }
        });

        stream
            .map(|res| {
                let pb = pb.clone();
                async move {
                    match res {
                        Ok((this, items)) => {
                            let batch_count = items.len() as u64;
                            let last_gid = items.last().map(|a| a.id());

                            if let Err(err) = this.ingest(items).await {
                                error!(
                                    "Ingest failed for {} batch ending at {:?}: {:?}",
                                    T::INDEX,
                                    last_gid,
                                    err
                                );
                            } else {
                                pb.inc(batch_count);
                            }
                        }
                        Err(err) => {
                            error!("Failed to fetch {} batch: {:?}", T::INDEX, err);
                        }
                    }
                }
            })
            .buffer_unordered(concurrency)
            .collect::<Vec<_>>()
            .await;

        Ok(())
    }

    pub async fn sync<T: QueryAble>(&self) -> Result<()> {
        let mut sizer = AdaptiveBatchSizer::new(T::batch_size(), Duration::from_secs(5));

        loop {
            let t0 = Instant::now();
            let Data { items } = T::query_unsynced(sizer.current(), &self.db).await?;
            let items = items.map(|items| items.0).unwrap_or_default();

            if items.is_empty() {
                break;
            }

            let ids: Vec<Uuid> = items.iter().map(|a| a.id()).collect();
            self.ingest(items).await?;
            T::update_syncs(&ids, &self.db).await?;
            sizer.adjust(t0.elapsed());
        }

        Ok(())
    }

    async fn ingest<T: QueryAble>(&self, items: Vec<T>) -> Result<()> {
        let ids: Vec<Uuid> = items.iter().map(|a| a.id()).collect();

        T::insert_sync_ids(&ids[..], &self.db).await?;
        let taskinfo = self.meili_client.add_item(items).await?;

        match self.meili_client.wait_for_task(taskinfo).await? {
            Status::Success => {
                T::update_syncs(&ids[..], &self.db).await?;
                info!("Batch ingested successfully ({} {})", ids.len(), T::INDEX);
            }
            Status::Failure => {
                error!("Failed to ingest batch ({} {})", ids.len(), T::INDEX);
            }
        }

        Ok(())
    }
}
