use mbmeta_db::{
    Rating,
    album::{Album, Image, Medium, Release, Track},
    artist::Artist,
};
use serde::{Deserialize, Serialize};
use url::Url;
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
    pub rating: RatingInfo,
    pub links: Vec<String>,
    pub genres: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "PascalCase")]
pub struct RatingInfo {
    pub count: Option<u32>,
    pub value: Option<f64>,
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
            links: value.links,
            genres: value.genres,
        }
    }
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
    pub rating: Option<RatingInfo>,
    pub links: Vec<Link>,
    pub genres: Option<Vec<String>>,
    pub images: Option<Vec<ImageInfo>>,
    pub releases: Option<Vec<ReleaseInfo>>,
}

// TODO: move these shenanigans to indexing and just expose as is
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

fn build_caa_url(release_gid: &str, image_id: u64) -> String {
    format!(
        "https://coverartarchive.org/release/{}/{}",
        release_gid, image_id
    )
}

fn build_image(image: &Image, id: &str, r#type: &str) -> ImageInfo {
    let remote_url = image.image_id.map(|image_id| build_caa_url(id, image_id));
    ImageInfo {
        cover_type: r#type.to_string(),
        url: remote_url
            .as_ref()
            .map(|url| format!("https://images.lidarr.audio/cache/{url}")),
        remote_url,
    }
}

fn extract_link_type(url: &str) -> Option<String> {
    let parsed = Url::parse(url).ok()?;
    let host = parsed.host_str()?;
    let host = host.strip_prefix("www.").unwrap_or(host);
    let parts: Vec<&str> = host.split('.').collect();
    parts.get(0).map(|s| s.to_string())
}

#[cfg(test)]
mod test {
    use crate::extract_link_type;

    #[test]
    fn test() {
        assert_eq!(
            extract_link_type("https://www.google.com/search"),
            Some("google".to_string())
        );
        assert_eq!(
            extract_link_type("https://subdomain.example.co.uk"),
            Some("subdomain".to_string())
        );
        assert_eq!(
            extract_link_type("http://github.com"),
            Some("github".to_string())
        );
        assert_eq!(
            extract_link_type("ftp://testsite.org"),
            Some("testsite".to_string())
        );
    }
}
