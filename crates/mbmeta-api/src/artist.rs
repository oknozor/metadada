use crate::ArtistInfo;
use crate::error::{AppError, AppResult};
use autometrics::autometrics;
use axum::extract::Path;
use axum::{Extension, Json};
use axum_macros::debug_handler;
use meilisearch_sdk::client::Client;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

#[debug_handler]
#[utoipa::path(
    get,
    path = "/{mbid}",
    summary = "Get artist info",
    responses(
        (status = 200, description = "Artist info with albums", body = ArtistInfo, content_type = "application/json"),
        (status = 400, description = "Invalid MBID"),
    ),
)]
#[autometrics]
pub async fn by_id(
    Path(mbid): Path<String>,
    Extension(client): Extension<Client>,
) -> AppResult<Json<ArtistInfo>> {
    Ok(Json(
        client
            .index("artists")
            .search()
            .with_filter(&format!("id = '{mbid}'"))
            .execute::<ArtistInfo>()
            .await?
            .hits
            .into_iter()
            .map(|r| r.result)
            .next()
            .ok_or(AppError::NotFound)?,
    ))
}

pub(crate) fn router() -> OpenApiRouter {
    OpenApiRouter::new().routes(routes!(by_id))
}
