use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    Rating,
    indexables::{
        RatingInfo,
        album::{ImageInfo, Link},
        extract_link_type,
    },
    queryables::artist::{AlbumLight, Artist},
};

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
    pub rating: RatingInfo,
    pub links: Vec<Link>,
    pub overview: Option<String>,
    pub albums: Vec<AlbumLightInfo>,
    #[serde(default = "default_images")]
    pub images: Option<Vec<ImageInfo>>,
    pub genres: Vec<String>,
}

fn default_images() -> Option<Vec<ImageInfo>> {
    Some(vec![])
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AlbumLightInfo {
    pub id: String,
    pub oldids: Vec<String>,
    pub title: String,
    pub r#type: String,
    pub releasestatuses: Vec<String>,
    pub secondarytypes: Vec<String>,
    pub releasedate: Option<String>,
    pub rating: Option<RatingInfo>,
}

impl From<AlbumLight> for AlbumLightInfo {
    fn from(value: AlbumLight) -> Self {
        Self {
            id: value.id.to_string(),
            oldids: value.oldids,
            title: value.title,
            r#type: value.r#type,
            releasestatuses: value.releasestatuses,
            secondarytypes: value.secondarytypes,
            releasedate: value.releasedate,
            rating: value.rating.map(Into::into),
        }
    }
}

impl From<Rating> for RatingInfo {
    fn from(value: Rating) -> Self {
        Self {
            count: value.count,
            value: value.value,
        }
    }
}

impl From<Artist> for ArtistInfo {
    fn from(value: Artist) -> Self {
        Self {
            id: value.id.to_string(),
            oldids: value.oldids.unwrap_or_default(),
            artistname: value.artistname,
            sortname: value.sortname,
            artistaliases: value.artistaliases,
            status: value.status,
            disambiguation: value.disambiguation,
            r#type: value.r#type,
            rating: RatingInfo {
                count: value.rating.count,
                value: value.rating.value,
            },
            links: value
                .links
                .into_iter()
                .filter_map(|link| extract_link_type(&link).zip(Some(link)))
                .map(|(target, r#type)| Link { target, r#type })
                .collect(),
            genres: value.genres,
            overview: None,
            images: Some(vec![]),
            albums: value
                .albums
                .map(|albums| albums.into_iter().map(AlbumLightInfo::from).collect())
                .unwrap_or_default(),
        }
    }
}
