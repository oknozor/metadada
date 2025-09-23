use metadada_db::indexables::{album::AlbumInfo, artist::ArtistInfo};
use serde::{Deserialize, Serialize};
use utoipa::{OpenApi, ToSchema};
use utoipa_axum::router::OpenApiRouter;

pub mod album;
pub mod artist;
pub mod error;
pub mod fingerprints;
pub mod recent;
pub mod search;

// TODO
#[derive(OpenApi)]
#[openapi(components(schemas(crate::error::AppError)))]
pub struct ApiDoc;

pub fn router() -> OpenApiRouter {
    OpenApiRouter::new()
        .nest("/album", album::router())
        .nest("/artist", artist::router())
        .nest("/recent", recent::router())
        .nest("/search", search::router())
        .nest("/search", fingerprints::router())
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ItemInfo {
    pub score: u32,
    pub artist: Option<ArtistInfo>,
    pub album: Option<AlbumInfo>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(untagged)]
pub enum Items {
    Artist(ArtistInfo),
    Album(AlbumInfo),
    Item(Box<ItemInfo>),
}
