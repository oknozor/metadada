use std::net::SocketAddr;

use autometrics::prometheus_exporter;
use axum::{Extension, routing::get};
use clap::Parser;
use mbmeta_api::ApiDoc;
use mbmeta_db::queryables::album::Album;
use mbmeta_meili::MeiliClient;
use mbmeta_pipeline::Ingestor;
use mbmeta_settings::Settings;
use sqlx::{PgPool, postgres::PgPoolOptions};
use tower_http::trace::TraceLayer;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;
use utoipa_swagger_ui::SwaggerUi;

#[derive(Parser, Debug)]
pub enum Cli {
    Init,
    Serve,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "tower_http=debug,mbmeta=debug,mbeta-pipeline=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = &Settings::get()?;

    info!("Connecting to musicbrainz database");
    let db = PgPoolOptions::new()
        .max_connections(5)
        .connect(&config.db_url())
        .await?;

    let meili_client = MeiliClient::new(&config.meili.url, &config.meili.api_key);

    let cli = Cli::parse();
    match cli {
        Cli::Init => initial_indexing(meili_client, db).await?,
        Cli::Serve => serve(config, meili_client).await?,
    }
    Ok(())
}

async fn serve(config: &Settings, meili_client: MeiliClient) -> anyhow::Result<()> {
    let addr = SocketAddr::from(([0, 0, 0, 0], config.api.port));
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    let app = mbmeta_api::router()
        .layer(TraceLayer::new_for_http())
        .layer(Extension(meili_client.client));

    let (router, api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .nest("/api/v0.4", app)
        .split_for_parts();

    let router = router
        .route(
            "/metrics",
            get(|| async { prometheus_exporter::encode_http_response() }),
        )
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", api.clone()));

    axum::serve(listener, router).await?;
    Ok(())
}

async fn initial_indexing(meili_client: MeiliClient, db: PgPool) -> anyhow::Result<()> {
    info!("Setting up MeiliSearch indexes");
    meili_client.setup_indexes().await?;

    let ingestor = Ingestor { db, meili_client };

    info!("Starting ingestor");
    // ingestor.batch_ingest::<Artist>(50_000).await?;
    ingestor.batch_ingest::<Album>(10_000).await?;

    Ok(())
}
