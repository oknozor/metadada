use std::pin::Pin;

use crate::{Data, Indexable, Rating};
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
}

impl Indexable for Album {
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
            sqlx::query(
                r#"
                INSERT INTO releases (id)
                VALUES (UNNEST($1::uuid[]))
                ON CONFLICT (id) DO NOTHING;
                "#,
            )
            .bind(ids)
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
            sqlx::query(
                r#"
                UPDATE releases
                SET sync = TRUE
                WHERE id = ANY($1::uuid[])
                "#,
            )
            .bind(ids)
            .execute(db)
            .await?;
            Ok(())
        })
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
