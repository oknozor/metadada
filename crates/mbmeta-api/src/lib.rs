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
#[serde(untagged)]
pub enum Items {
    Artist(ArtistInfo),
    Album(AlbumInfo),
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ArtistInfo {
    pub id: String,
    pub oldids: Vec<String>,
    pub artistname: String,
    pub sortname: String,
    pub artistaliases: Vec<String>,
    pub status: String,
    pub disambiguation: String,
    pub r#type: Option<String>,
    pub rating: Rating,
    pub links: Vec<String>,
    pub genres: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "PascalCase")]
pub struct Rating {
    pub count: Option<u32>,
    pub value: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AlbumInfo {
    pub id: String,
    pub oldids: Option<Vec<String>>,
    pub disambiguation: Option<String>,
    pub title: String,
    pub aliases: Vec<String>,
    pub r#type: String,
    pub secondarytypes: Option<Vec<String>>,
    pub releasedate: Option<String>,
    pub artistid: Option<String>,
    pub artistids: Option<Vec<String>>,
    pub rating: Option<Rating>,
    pub links: Option<Vec<String>>,
    pub genres: Option<Vec<String>>,
    pub images: Option<Vec<Image>>,
    pub releases: Option<Vec<Release>>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "PascalCase")]
pub struct Image {
    pub cover_type: String,
    pub url: Option<String>,
    #[serde(rename = "remoteUrl")]
    pub remote_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct Release {
    pub id: String,
    pub oldids: Option<Vec<String>>,
    pub title: String,
    pub disambiguation: Option<String>,
    pub status: Option<String>,
    pub releasedate: Option<String>,
    pub label: Option<Vec<String>>,
    pub country: Option<Vec<String>>,
    pub media: Option<Vec<Medium>>,
    pub track_count: Option<u32>,
    pub tracks: Option<Vec<Track>>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct Medium {
    pub format: Option<String>,
    pub name: Option<String>,
    pub position: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct Track {
    pub id: String,
    pub oldids: Option<Vec<String>>,
    pub recordingid: Option<String>,
    pub oldrecordingids: Option<Vec<String>>,
    pub artistid: Option<String>,
    pub trackname: Option<String>,
    pub durationms: Option<u32>,
    pub mediumnumber: Option<u32>,
    pub tracknumber: Option<String>,
    pub trackposition: Option<u32>,
}
