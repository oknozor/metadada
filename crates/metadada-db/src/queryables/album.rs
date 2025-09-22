use std::pin::Pin;

use crate::indexables::album::AlbumInfo;
use crate::queryables::QueryAble;
use crate::queryables::artist::Artist;
use crate::{Data, Rating};
use metadada_settings::ALBUM_BATCH_SIZE;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use sqlx::types::Json;
use uuid::Uuid;

pub async fn count_albums(db: &PgPool) -> Result<i64, sqlx::Error> {
    let rec = sqlx::query!("SELECT COUNT(*) as count FROM release_group")
        .fetch_one(db)
        .await?;
    Ok(rec.count.unwrap_or(0))
}

pub async fn all_albums(
    last_seen_gid: Option<Uuid>,
    limit: i64,
    db: &PgPool,
) -> Result<Data<Album>, sqlx::Error> {
    sqlx::query_file_as!(Data, "queries/all_release_group.sql", last_seen_gid, limit)
        .fetch_one(db)
        .await
}
pub async fn unsynced_albums(limit: i64, db: &PgPool) -> Result<Data<Album>, sqlx::Error> {
    sqlx::query_file_as!(Data, "queries/unsynced_release_group.sql", limit)
        .fetch_one(db)
        .await
}

async fn unsynced_releases_count(db: &PgPool) -> sqlx::Result<i64> {
    sqlx::query_scalar!("SELECT COUNT(*) FROM metadada.releases_sync WHERE sync IS FALSE")
        .fetch_one(db)
        .await
        .map(|c| c.unwrap_or_default())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Album {
    pub id: Uuid,
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
    pub artists: Option<Vec<Artist>>,
}

impl QueryAble for Album {
    type Indexable = AlbumInfo;

    const INDEX: &'static str = "albums";
    const ID: &'static str = "id";

    fn id(&self) -> Uuid {
        self.id
    }

    fn query_all<'a>(
        last_seen_gid: Option<uuid::Uuid>,
        limit: i64,
        db: &'a sqlx::PgPool,
    ) -> Pin<Box<dyn Future<Output = Result<crate::Data<Self>, sqlx::Error>> + Send + 'a>> {
        Box::pin(all_albums(last_seen_gid, limit, db))
    }

    fn query_unsynced<'a>(
        limit: i64,
        db: &'a PgPool,
    ) -> Pin<Box<dyn Future<Output = Result<crate::Data<Self>, sqlx::Error>> + Send + 'a>> {
        Box::pin(unsynced_albums(limit, db))
    }

    fn unsynced_count<'a>(
        db: &'a PgPool,
    ) -> Pin<Box<dyn Future<Output = Result<i64, sqlx::Error>> + Send + 'a>> {
        Box::pin(unsynced_releases_count(db))
    }

    fn count<'a>(
        db: &'a PgPool,
    ) -> Pin<Box<dyn Future<Output = Result<i64, sqlx::Error>> + Send + 'a>> {
        Box::pin(count_albums(db))
    }

    fn insert_sync_ids<'a>(
        ids: &'a [Uuid],
        db: &'a PgPool,
    ) -> Pin<Box<dyn Future<Output = Result<(), sqlx::Error>> + Send + 'a>> {
        Box::pin(async move {
            sqlx::query!(
                r#"
                INSERT INTO metadada.releases_sync (id)
                VALUES (UNNEST($1::uuid[]))
                ON CONFLICT (id) DO NOTHING;
                "#,
                ids
            )
            .execute(db)
            .await?;
            Ok(())
        })
    }

    fn update_syncs<'a>(
        ids: &'a [Uuid],
        db: &'a PgPool,
    ) -> Pin<Box<dyn Future<Output = Result<(), sqlx::Error>> + Send + 'a>> {
        Box::pin(async move {
            sqlx::query!(
                r#"
                UPDATE metadada.releases_sync
                SET sync = TRUE
                WHERE id = ANY($1::uuid[])
                "#,
                ids
            )
            .execute(db)
            .await?;
            Ok(())
        })
    }

    fn to_model(self) -> Self::Indexable {
        AlbumInfo::from(self)
    }

    fn batch_size() -> i64 {
        *ALBUM_BATCH_SIZE
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Image {
    pub r#type: Option<String>,
    pub release_gid: Option<String>,
    pub image_id: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Medium {
    pub format: Option<String>,
    pub name: Option<String>,
    pub position: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
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

#[cfg(test)]
mod test {
    use sqlx::postgres::PgPoolOptions;
    use uuid::Uuid;

    use crate::queryables::{QueryAble, artist::Artist};

    #[tokio::test]
    async fn test() {
        let db = PgPoolOptions::new()
            .max_connections(5)
            .connect("postgres://musicbrainz:musicbrainz@localhost:5432/musicbrainz")
            .await
            .unwrap();

        let a = Artist::query_all(Some(Uuid::nil()), 3, &db).await.unwrap();
        println!("{:?}", a);
    }
}
