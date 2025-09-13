use std::pin::Pin;

use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{Data, QueryAble, Rating, indexables::artist::ArtistInfo};
use sqlx::types::Json;

#[derive(Debug, Serialize, Deserialize)]
pub struct Artist {
    pub id: uuid::Uuid,
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

pub async fn count_artists(db: &PgPool) -> Result<i64, sqlx::Error> {
    let rec = sqlx::query!("SELECT COUNT(*) as count FROM artist")
        .fetch_one(db)
        .await?;
    Ok(rec.count.unwrap_or(0))
}

pub async fn all_artists(
    last_seen_gid: Option<Uuid>,
    limit: i64,
    db: &PgPool,
) -> Result<Data<Artist>, sqlx::Error> {
    sqlx::query_file_as!(Data, "queries/all_artists.sql", last_seen_gid, limit)
        .fetch_one(db)
        .await
}

impl QueryAble for Artist {
    type Indexable = ArtistInfo;
    const INDEX: &'static str = "artists";
    const ID: &'static str = "id";

    fn id(&self) -> Uuid {
        self.id
    }

    fn query_all<'a>(
        last_seen_gid: Option<uuid::Uuid>,
        limit: i64,
        db: &'a sqlx::PgPool,
    ) -> Pin<Box<dyn Future<Output = Result<crate::Data<Self>, sqlx::Error>> + Send + 'a>> {
        Box::pin(all_artists(last_seen_gid, limit, db))
    }

    fn count<'a>(
        db: &'a PgPool,
    ) -> Pin<Box<dyn Future<Output = Result<i64, sqlx::Error>> + Send + 'a>> {
        Box::pin(count_artists(db))
    }

    fn insert_sync_ids<'a>(
        ids: &'a [Uuid],
        db: &'a PgPool,
    ) -> Pin<Box<dyn Future<Output = Result<(), sqlx::Error>> + Send + 'a>> {
        Box::pin(async move {
            sqlx::query(
                r#"
                INSERT INTO artists_sync (id)
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
                UPDATE artists_sync
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

    fn to_model(self) -> Self::Indexable {
        ArtistInfo::from(self)
    }
}
