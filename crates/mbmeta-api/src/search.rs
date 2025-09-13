use crate::error::AppResult;
use crate::{AlbumInfo, ArtistInfo, ItemInfo, Items};
use autometrics::autometrics;
use axum::extract::Query;
use axum::{Extension, Json};
use axum_macros::debug_handler;
use futures::join;
use meilisearch_sdk::client::Client;
use serde::Deserialize;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SearchQuery {
    pub r#type: QueryType,
    pub query: String,
    pub include_tracks: Option<u8>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum QueryType {
    Artist,
    Album,
    All,
}

#[debug_handler]
#[utoipa::path(
    get,
    path = "/",
    params(
        ("type", description = "'artist', 'album', 'all'", example = "all"),
        ("query", description = "full text search query", example = "Joy Division"),
    ),
    summary = "Artist by id",
    responses(
        (status = 200, description = "An artist", body = Vec<Items>, content_type = "application/json"),
    ),
)]
#[autometrics]
pub async fn search(
    Query(q): Query<SearchQuery>,
    Extension(client): Extension<Client>,
) -> AppResult<Json<Vec<Items>>> {
    match q.r#type {
        QueryType::Artist => search_artists(&client, &q.query, 10)
            .await
            .map(|artists| artists.into_iter().map(Items::Artist).collect::<Vec<_>>())
            .map(Json),
        QueryType::Album => search_albums(&client, &q.query, 10)
            .await
            .map(|albums| albums.into_iter().map(Items::Album).collect::<Vec<_>>())
            .map(Json),
        QueryType::All => {
            let artists = search_artists(&client, &q.query, 5);
            let albums = search_albums(&client, &q.query, 5);
            let (artists, albums) = join!(artists, albums);
            let mut all = artists?
                .into_iter()
                .map(|artist| {
                    Items::Item(Box::new(ItemInfo {
                        score: 1,
                        artist: Some(artist),
                        album: None,
                    }))
                })
                .collect::<Vec<_>>();

            let albums = albums?
                .into_iter()
                .map(|album| {
                    Items::Item(Box::new(ItemInfo {
                        score: 1,
                        artist: None,
                        album: Some(album),
                    }))
                })
                .collect::<Vec<_>>();

            all.extend(albums);
            Ok(Json(all))
        }
    }
}

async fn search_artists(client: &Client, query: &str, limit: usize) -> AppResult<Vec<ArtistInfo>> {
    Ok(client
        .index("artists")
        .search()
        .with_limit(limit)
        .with_query(query)
        .execute::<ArtistInfo>()
        .await?
        .hits
        .into_iter()
        .map(|r| r.result)
        .collect::<Vec<_>>())
}

async fn search_albums(client: &Client, query: &str, limit: usize) -> AppResult<Vec<AlbumInfo>> {
    Ok(client
        .index("albums")
        .search()
        .with_limit(limit)
        .with_query(query)
        .execute::<AlbumInfo>()
        .await?
        .hits
        .into_iter()
        .map(|r| r.result)
        .collect::<Vec<_>>())
}

pub(crate) fn router() -> OpenApiRouter {
    OpenApiRouter::new().routes(routes!(search))
}
