use metadada_db::queryables::{QueryAble, album::Album, artist::Artist};
use metadada_pipeline::Ingestor;
use sqlx::PgPool;
use tokio::{select, sync::mpsc::Receiver};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

pub struct MusicbrainzPgListener {
    pool: PgPool,
    rx: Receiver<()>,
    ingestor: Ingestor,
    cancellation_token: CancellationToken,
}

impl MusicbrainzPgListener {
    pub async fn create(
        ingestor: Ingestor,
        pool: PgPool,
        rx: Receiver<()>,
        cancellation_token: CancellationToken,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            ingestor,
            pool,
            rx,
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
        info!("Starting reindex command listener");

        let mut retry_count = 3;

        while retry_count > 0 {
            while let Some(()) = self.rx.recv().await {
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

            warn!("pg listener failed, trying to reconnect");
            retry_count -= 1;
        }

        error!("Index command listener exited");
        Ok(())
    }
}
