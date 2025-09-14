use anyhow::Result;
use futures::stream::{self, StreamExt};
use indicatif::{ProgressBar, ProgressStyle};
use mbmeta_db::{Data, queryables::QueryAble};
use mbmeta_meili::{MeiliClient, Status};
use sqlx::{PgPool, types::Uuid};
use tracing::{error, info};

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
            )
            .unwrap()
            .progress_chars("#>-"),
        );

        let stream = stream::unfold(last_seen_gid, |last_gid| async move {
            let this = self.clone();

            match T::query_all(last_gid, T::batch_size(), &this.db).await {
                Ok(Data { items }) => {
                    let items: Vec<T> = match items {
                        Some(a) if !a.is_empty() => a.0,
                        _ => return None,
                    };

                    let next_gid = items.last().map(|a| a.id());

                    Some((Ok((this, items)), next_gid))
                }
                Err(e) => Some((Err(e), last_gid)),
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
        loop {
            let Data { items } = T::query_unsynced(T::batch_size(), &self.db).await?;
            let items = items.map(|items| items.0).unwrap_or_default();

            if items.is_empty() {
                break;
            }

            let ids: Vec<Uuid> = items.iter().map(|a| a.id()).collect();
            self.ingest(items).await?;
            T::update_syncs(&ids, &self.db).await?;
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
