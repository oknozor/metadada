use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    indexables::{RatingInfo, build_image, extract_link_type},
    queryables::{
        album::{Album, Medium, Release, Track},
        artist::Artist,
    },
};

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
    pub rating: Option<RatingInfo>,
    pub links: Vec<Link>,
    pub genres: Option<Vec<String>>,
    pub images: Option<Vec<ImageInfo>>,
    #[serde(rename = "Releases")]
    pub releases: Option<Vec<ReleaseInfo>>,
    pub releasestatuses: Option<Vec<String>>,
    pub overview: Option<String>,
    pub artists: Vec<ArtistLightInfo>,
}

impl From<Album> for AlbumInfo {
    fn from(value: Album) -> Self {
        let id = value.id.to_string();
        let mut images = Vec::with_capacity(2);
        let cover = value.images.as_ref().and_then(|i| {
            i.iter()
                .find(|image| image.r#type == Some("Front".to_string()))
                .map(|image| build_image(image, &id, "Cover"))
        });

        if let Some(cover) = cover {
            images.push(cover);
        }

        let disc = value.images.as_ref().and_then(|i| {
            i.iter()
                .find(|image| image.r#type == Some("Medium".to_string()))
                .map(|image| build_image(image, &id, "Disc"))
        });

        if let Some(disc) = disc {
            images.push(disc);
        }

        Self {
            id,
            oldids: value.oldids,
            disambiguation: value.disambiguation,
            title: value.title,
            aliases: value.aliases,
            r#type: value.r#type,
            secondarytypes: value.secondarytypes,
            releasedate: value.releasedate,
            artistid: value.artistid,
            artistids: value.artistids,
            rating: value.rating.map(Into::into),
            links: value
                .links
                .unwrap_or_default()
                .into_iter()
                .filter_map(|link| extract_link_type(&link).zip(Some(link)))
                .map(|(target, r#type)| Link { target, r#type })
                .collect(),
            genres: value.genres,
            images: Some(images),
            releases: value
                .releases
                .map(|releases| releases.into_iter().map(Into::into).collect()),
            releasestatuses: None,
            overview: None,
            artists: value
                .artists
                .unwrap_or_default()
                .into_iter()
                .map(ArtistLightInfo::from)
                .collect::<Vec<_>>(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct Link {
    pub target: String,
    pub r#type: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ImageInfo {
    #[serde(rename = "CoverType")]
    pub cover_type: String,
    #[serde(rename = "Url")]
    pub url: Option<String>,
    #[serde(rename = "remoteUrl")]
    pub remote_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "PascalCase")]
pub struct ReleaseInfo {
    pub id: String,
    pub oldids: Option<Vec<String>>,
    pub title: String,
    pub disambiguation: Option<String>,
    pub status: Option<String>,
    pub releasedate: Option<String>,
    pub label: Option<Vec<String>>,
    pub country: Option<Vec<String>>,
    pub media: Option<Vec<MediumInfo>>,
    pub track_count: Option<u32>,
    pub tracks: Option<Vec<TrackInfo>>,
}

impl From<Release> for ReleaseInfo {
    fn from(value: Release) -> Self {
        Self {
            id: value.id,
            oldids: value.oldids,
            title: value.title,
            disambiguation: value.disambiguation,
            status: value.status,
            releasedate: value.releasedate,
            label: value.label,
            country: value.country,
            media: Some(
                value
                    .media
                    .unwrap_or_default()
                    .into_iter()
                    .map(Into::into)
                    .collect(),
            ),
            track_count: value.track_count,
            tracks: Some(
                value
                    .tracks
                    .unwrap_or_default()
                    .into_iter()
                    .map(Into::into)
                    .collect(),
            ),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MediumInfo {
    pub format: Option<String>,
    pub name: Option<String>,
    pub position: Option<u32>,
}

impl From<Medium> for MediumInfo {
    fn from(value: Medium) -> Self {
        Self {
            format: value.format,
            name: value.name,
            position: value.position,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub struct TrackInfo {
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

impl From<Track> for TrackInfo {
    fn from(value: Track) -> Self {
        Self {
            id: value.id.to_string(),
            oldids: value.oldids,
            recordingid: value.recordingid,
            oldrecordingids: value.oldrecordingids,
            artistid: value.artistid,
            trackname: value.trackname,
            durationms: value.durationms,
            mediumnumber: value.mediumnumber,
            tracknumber: value.tracknumber,
            trackposition: value.trackposition,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ArtistLightInfo {
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
    pub genres: Vec<String>,
    #[serde(default = "default_images")]
    pub images: Option<Vec<ImageInfo>>,
    #[serde(default = "default_albums")]
    pub albums: Option<Vec<String>>,
}

fn default_images() -> Option<Vec<ImageInfo>> {
    Some(vec![])
}

fn default_albums() -> Option<Vec<String>> {
    Some(vec![])
}

impl From<Artist> for ArtistLightInfo {
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
            albums: Some(vec![]),
        }
    }
}
