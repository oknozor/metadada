use mbmeta_model::Artist;
use sqlx::{PgPool, prelude::FromRow, types::Json};

pub async fn all_artists(offset: i64, limit: i64, db: &PgPool) -> Result<Data, sqlx::Error> {
    sqlx::query_file_as!(Data, "queries/all_artists.sql", offset, limit)
        .fetch_one(db)
        .await
}

#[derive(FromRow)]
pub struct Data2 {
    pub artist: Json<Vec<Artist>>,
}
#[derive(FromRow)]
pub struct Data {
    pub artist: Option<Json<Vec<Artist>>>,
}
