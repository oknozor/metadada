use mbmeta_db::artist::Data;
use mbmeta_meili::MeiliClient;
use mbmeta_model::Artist;
use sqlx::{PgPool, query, types::Uuid};
use tokio::spawn;
use tracing::{error, info};

#[derive(Clone)]
pub struct Ingestor {
    pub mb_db: PgPool,
    pub sync_db: PgPool,
    pub meili_client: MeiliClient,
}

impl Ingestor {
    pub async fn batch_ingest_artists(&self) -> anyhow::Result<()> {
        // TODO: store offset on fail
        let mut offset = 810000;
        let limit = 15000;

        loop {
            info!(" artist offset {}", offset);
            let Data { artist } =
                mbmeta_db::artist::all_artists(offset, limit, &self.mb_db).await?;
            let artists = match artist {
                Some(a) if !a.is_empty() => a.0,
                _ => break,
            };

            info!("Ingesting batch with offset {offset}");
            let artists = artists.into_iter().map(Artist::from).collect::<Vec<_>>();
            self.ingest_artist(artists.as_slice()).await?;
            offset += limit;
        }

        Ok(())
    }

    async fn ingest_artist(&self, artists: &[Artist]) -> anyhow::Result<()> {
        let artist_ids: Vec<Uuid> = artists.iter().map(|a| a.id.clone()).collect();

        query!(
            r#"
            INSERT INTO artists (id)
            VALUES (UNNEST($1::uuid[]))
            ON CONFLICT (id) DO NOTHING;
            "#,
            &artist_ids[..]
        )
        .execute(&self.sync_db)
        .await?;

        let taskinfo = self.meili_client.add_artists(artists).await?;
        let artist_ids = artist_ids.clone();
        let self_ = self.clone();
        spawn(async move {
            let task = self_.meili_client.wait_for_task(taskinfo).await?;
            match task {
                mbmeta_meili::Status::Success => {
                    sqlx::query!(
                        r#"
                                UPDATE artists
                                SET sync = TRUE
                                WHERE id = ANY($1::uuid[])
                                "#,
                        &artist_ids[..] // Vec<Uuid>
                    )
                    .execute(&self_.sync_db)
                    .await?;
                    info!("Batch ingested")
                }
                mbmeta_meili::Status::Failure => {
                    error!("Failed to ingest artists batch");
                }
            }
            anyhow::Ok(())
        });

        Ok(())
    }
}
