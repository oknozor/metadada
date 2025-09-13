use serde::{Deserialize, Serialize};
use url::Url;
use utoipa::ToSchema;

use crate::{indexables::album::ImageInfo, queryables::album::Image};

pub mod album;
pub mod artist;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "PascalCase")]
pub struct RatingInfo {
    pub count: Option<u32>,
    pub value: Option<f64>,
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
    parts.first().map(|s| s.to_string())
}

#[cfg(test)]
mod test {
    use crate::indexables::extract_link_type;

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
