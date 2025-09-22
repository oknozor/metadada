use std::net::SocketAddr;

use autometrics::prometheus_exporter;
use axum::{Extension, routing::get};
use clap::{Parser, builder::PossibleValuesParser};
use metadada_api::ApiDoc;
use metadada_db::queryables::{album::Album, artist::Artist};
use metadada_meili::MeiliClient;
use metadada_pipeline::Ingestor;
use metadada_settings::Settings;
use metadada_importer::MbLight;
use sqlx::{PgPool, postgres::PgPoolOptions};
use tokio::signal::unix::{SignalKind, signal};
use tokio_util::sync::CancellationToken;
use tower_http::trace::TraceLayer;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;
use utoipa_swagger_ui::SwaggerUi;

#[derive(Parser, Debug)]
pub enum Cli {
    Init {
        #[arg(
            long,
            short,
            value_parser = PossibleValuesParser::new(["albums", "artists"]),
            default_values = ["artists", "albums"],
            help = "Name of the indexes to sync"
        )]
        index: Vec<String>,
    },
    Serve,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| {
                "tower_http=debug,metadada=debug,mbeta-pipeline=debug,metadada_importer=debug".into()
            }),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Settings::get()?;

    info!("Connecting to musicbrainz database");
    let db = PgPoolOptions::new()
        .max_connections(5)
        .connect(&config.db_url())
        .await?;

    let meili_client = MeiliClient::new(&config.meili.url, &config.meili.api_key);
    let mut mblight = MbLight::new(config.clone(), db.clone()).await?;

    if !mblight.has_data().await? {
        let local_path = metadada_importer::downloader::github::download_musicbrainz_sql().await?;
        mblight.create_schemas().await?;
        mblight.create_tables(&local_path).await?;
        mblight.ingest_musicbrainz_data().await?;
        mblight.run_all_scripts(local_path).await?;
    }

    sqlx::migrate!("../../migrations").run(&db).await?;

    let cli = Cli::parse();
    match cli {
        Cli::Init { index } => initial_indexing(meili_client, db, &index).await?,
        Cli::Serve => serve(config, meili_client, mblight, db).await?,
    }
    Ok(())
}

async fn serve(
    config: Settings,
    meili_client: MeiliClient,
    mblight: MbLight,
    db: PgPool,
) -> anyhow::Result<()> {
    let addr = SocketAddr::from(([0, 0, 0, 0], config.api.port));
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    let app = metadada_api::router()
        .layer(TraceLayer::new_for_http())
        .layer(Extension(meili_client.client.clone()));

    let (router, api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .nest("/api/v1", app)
        .split_for_parts();

    let router = router
        .route(
            "/metrics",
            get(|| async { prometheus_exporter::encode_http_response() }),
        )
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", api.clone()));

    let token = CancellationToken::new();
    let token_clone = token.clone();

    tokio::spawn(async move {
        let mut sigterm =
            signal(SignalKind::terminate()).expect("Failed to register SIGTERM handler");
        sigterm.recv().await;
        info!("Received SIGTERM, shutting down...");
        token_clone.cancel();
    });

    let ingestor = Ingestor {
        db: db.clone(),
        meili_client: meili_client.clone(),
    };

    let mut pg_listener =
        metadada_pg_listener::MusicbrainzPgListener::create(ingestor, db.clone(), token.clone())
            .await?;

    let axum_token = token.clone();
    let server = axum::serve(listener, router).with_graceful_shutdown(async move {
        axum_token.cancelled().await;
    });
    // TODO: pass cancellation token here
    let pg_listener_task = pg_listener.run();
    let live_data_feed_task = mblight.apply_all_pending_replication();

    tokio::select! {
        result = server => {
            info!("Server shutdown: {:?}", result);
            result?;
        },
        result = pg_listener_task => {
            info!("Pg Listener shutdown: {:?}", result);
            result?;
        },
        result = live_data_feed_task => {
            info!("Live Data Feed worker shutdown: {:?}", result);
            result?;
        },
        _ = token.cancelled() => {
            info!("Shutdown signal received");
        }
    }
    Ok(())
}

async fn initial_indexing(
    meili_client: MeiliClient,
    db: PgPool,
    indexes: &[String],
) -> anyhow::Result<()> {
    info!("Setting up MeiliSearch indexes");

    let ingestor = Ingestor {
        db,
        meili_client: meili_client.clone(),
    };

    info!("Starting ingestor");
    for index in indexes {
        match index.as_str() {
            "artists" => {
                meili_client.setup_artist_index().await?;
                ingestor.batch_ingest::<Artist>().await?;
            }
            "albums" => {
                meili_client.setup_album_index().await?;
                ingestor.batch_ingest::<Album>().await?;
            }
            _ => unreachable!(),
        }
    }

    Ok(())
}
