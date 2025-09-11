use anyhow::Result;
use futures::stream::{self, StreamExt};
use indicatif::{ProgressBar, ProgressStyle};
use mbmeta_db::{Data, Indexable};
use mbmeta_meili::{MeiliClient, Status};
use sqlx::{PgPool, types::Uuid};
use tracing::{error, info};

#[derive(Clone)]
pub struct Ingestor {
    pub mb_db: PgPool,
    pub sync_db: PgPool,
    pub meili_client: MeiliClient,
}

impl Ingestor {
    pub async fn batch_ingest<T: Indexable>(&self) -> Result<()> {
        let limit = 50_000;
        let concurrency = 10;
        let last_seen_gid: Option<Uuid> = Some(Uuid::nil());
        let total_artists: i64 = T::count(&self.mb_db).await?;
        let pb = ProgressBar::new(total_artists as u64);
        pb.set_style(
            ProgressStyle::with_template(
                "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})",
            )
            .unwrap()
            .progress_chars("#>-"),
        );

        let stream = stream::unfold(last_seen_gid, |last_gid| async move {
            let this = self.clone();

            match T::query_all(last_gid, limit, &this.mb_db).await {
                Ok(Data { items }) => {
                    let artists: Vec<T> = match items {
                        Some(a) if !a.is_empty() => a.0,
                        _ => return None,
                    };

                    let next_gid = artists.last().map(|a| a.id());

                    Some((Ok((this, artists)), next_gid))
                }
                Err(e) => Some((Err(e), last_gid)),
            }
        });

        stream
            .map(|res| {
                let pb = pb.clone();
                async move {
                    match res {
                        Ok((this, artists)) => {
                            let batch_count = artists.len() as u64;
                            let last_gid = artists.last().map(|a| a.id());

                            if let Err(err) = this.ingest(&artists).await {
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

    async fn ingest<T: Indexable>(&self, items: &[T]) -> Result<()> {
        let ids: Vec<Uuid> = items.iter().map(|a| a.id()).collect();

        T::insert_sync_ids(&ids[..], &self.sync_db).await?;
        let taskinfo = self.meili_client.add_item(items).await?;

        match self.meili_client.wait_for_task(taskinfo).await? {
            Status::Success => {
                T::update_syncs(&ids[..], &self.sync_db).await?;
                info!("Batch ingested successfully ({} {})", ids.len(), T::INDEX);
            }
            Status::Failure => {
                error!("Failed to ingest batch ({} {})", ids.len(), T::INDEX);
            }
        }

        Ok(())
    }
}
