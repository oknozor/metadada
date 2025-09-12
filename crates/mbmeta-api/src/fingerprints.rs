use crate::AlbumInfo;
use crate::error::AppResult;
use autometrics::autometrics;
use axum::Extension;
use axum::extract::Json;
use axum_macros::debug_handler;
use meilisearch_sdk::client::Client;
use serde::Deserialize;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

#[derive(Debug, Deserialize, ToSchema)]
pub struct FingerprintRequest(pub Vec<String>);

#[debug_handler]
#[utoipa::path(
    post,
    path = "/fingerprint",
    request_body = FingerprintRequest,
    summary = "Search albums by recording IDs",
    responses(
        (status = 200, description = "Albums info", body = Vec<AlbumInfo>, content_type = "application/json"),
        (status = 400, description = "Bad Request"),
    ),
)]
#[autometrics]
pub async fn search_fingerprint(
    Extension(client): Extension<Client>,
    Json(fingerprints): Json<FingerprintRequest>,
) -> AppResult<Json<Vec<AlbumInfo>>> {
    let ids = fingerprints
        .0
        .iter()
        .map(|id| format!("'{}'", id))
        .collect::<Vec<_>>()
        .join(", ");

    Ok(Json(
        client
            .index("albums")
            .search()
            .with_filter(&format!("id IN [{ids}]"))
            .execute::<AlbumInfo>()
            .await?
            .hits
            .into_iter()
            .map(|r| r.result)
            .collect::<Vec<_>>(),
    ))
}

pub(crate) fn router() -> OpenApiRouter {
    OpenApiRouter::new().routes(routes!(search_fingerprint))
}
