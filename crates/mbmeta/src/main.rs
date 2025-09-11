use mbmeta_db::{album::Album, artist::Artist};
use mbmeta_meili::MeiliClient;
use mbmeta_pipeline::Ingestor;
use mbmeta_settings::Settings;
use sqlx::postgres::PgPoolOptions;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "mbmeta=debug,mbeta-pipeline=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = &Settings::get()?;

    info!("Connecting to musicbrainz database");
    let mb_db = PgPoolOptions::new()
        .max_connections(5)
        .connect(&config.musicbrainz_db_url())
        .await?;

    info!("Connecting to sync database");
    let sync_db = PgPoolOptions::new()
        .max_connections(5)
        .connect(&config.sync_db_url())
        .await?;

    let meili_client = MeiliClient::new(&config.meili.url, &config.meili.api_key);

    info!("Setting up MeiliSearch indexes");
    meili_client.setup_indexes().await?;

    let ingestor = Ingestor {
        mb_db,
        sync_db,
        meili_client,
    };

    info!("Starting ingestor");
    ingestor.batch_ingest::<Artist>().await?;
    ingestor.batch_ingest::<Album>().await?;

    Ok(())
}
