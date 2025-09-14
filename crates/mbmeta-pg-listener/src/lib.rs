use mbmeta_db::queryables::{QueryAble, album::Album, artist::Artist};
use mbmeta_pipeline::Ingestor;
use sqlx::{PgPool, postgres::PgListener};
use tokio::select;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

pub struct MusicbrainzPgListener {
    pool: PgPool,
    pg_listener: PgListener,
    ingestor: Ingestor,
    cancellation_token: CancellationToken,
}

impl MusicbrainzPgListener {
    pub async fn create(
        ingestor: Ingestor,
        pool: PgPool,
        cancellation_token: CancellationToken,
    ) -> anyhow::Result<Self> {
        let pg_listener = PgListener::connect_with(&pool).await?;
        Ok(Self {
            ingestor,
            pool,
            pg_listener,
            cancellation_token,
        })
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        let token = self.cancellation_token.clone();
        select! {
            _ = token.cancelled() => Ok(()),
            result = self.listen() => result,
        }
    }

    async fn listen(&mut self) -> anyhow::Result<()> {
        info!("Starting PgListener");
        self.pg_listener.listen("replication_finished").await?;

        let mut retry_count = 3;

        while retry_count > 0 {
            while let Ok(notification) = self.pg_listener.recv().await {
                if notification.channel() == "replication_finished" {
                    let release_count = Artist::unsynced_count(&self.pool).await?;
                    let artist_count = Album::unsynced_count(&self.pool).await?;
                    info!(
                        "Musicbrainz live datafeed ingested: unsynced releases: {}, unsynced artists: {}",
                        release_count, artist_count
                    );
                    info!("Starting updating index for {} artists", artist_count);
                    self.ingestor.sync::<Artist>().await?;
                    info!("Starting updating index for {} albums", artist_count);
                    self.ingestor.sync::<Album>().await?;
                }
            }

            warn!("pg listener failed, trying to reconnect");
            retry_count -= 1;
        }

        error!("PgListener exited");
        Ok(())
    }
}
