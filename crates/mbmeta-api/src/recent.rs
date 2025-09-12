use crate::Items;
use crate::error::AppResult;
use autometrics::autometrics;
use axum::Json;
use axum::extract::Query;
use axum_macros::debug_handler;
use serde::Deserialize;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RecentQuery {
    pub since: Option<u64>, // timestamp in seconds
}

#[debug_handler]
#[utoipa::path(
    get,
    path = "/artist",
    params(
        ("since", description = "Unix timestamp for filtering recent updates", example = 0)
    ),
    summary = "Recently updated artists",
    responses(
        (status = 200, description = "List of recently updated artists", body = Vec<Items>, content_type = "application/json"),
    ),
)]
#[autometrics]
pub async fn get_recently_updated_artists(
    Query(q): Query<RecentQuery>,
) -> AppResult<Json<Vec<Items>>> {
    todo!()
}

#[debug_handler]
#[utoipa::path(
    get,
    path = "/album",
    params(
        ("since", description = "Unix timestamp for filtering recent updates", example = 0)
    ),
    summary = "Recently updated albums",
    responses(
        (status = 200, description = "List of recently updated albums", body = Vec<Items>, content_type = "application/json"),
    ),
)]
#[autometrics]
pub async fn get_recently_updated_albums(
    Query(q): Query<RecentQuery>,
) -> AppResult<Json<Vec<Items>>> {
    todo!()
}

pub(crate) fn router() -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(get_recently_updated_albums))
        .routes(routes!(get_recently_updated_artists))
}
