use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    Rating,
    indexables::{
        RatingInfo,
        album::{AlbumInfo, Link},
        extract_link_type,
    },
    queryables::artist::Artist,
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
    #[serde(rename = "Albums")]
    pub albums: Option<Vec<AlbumInfo>>,
    pub genres: Vec<String>,
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
            oldids: value.oldids,
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
            albums: None,
        }
    }
}
