use std::pin::Pin;

use mbmeta_settings::Settings;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use sqlx::{PgPool, prelude::FromRow, types::Json};
use uuid::Uuid;

pub mod album;
pub mod artist;

pub trait Indexable: DeserializeOwned + Serialize + Send + Sync + Sized {
    const INDEX: &'static str;
    const ID: &'static str;

    fn id(&self) -> Uuid;

    fn query_all<'a>(
        last_seen_gid: Option<Uuid>,
        limit: i64,
        db: &'a PgPool,
    ) -> Pin<Box<dyn Future<Output = Result<crate::Data<Self>, sqlx::Error>> + Send + 'a>>;

    fn count<'a>(
        db: &'a PgPool,
    ) -> Pin<Box<dyn Future<Output = Result<i64, sqlx::Error>> + Send + 'a>>;

    fn insert_sync_ids<'a>(
        ids: &'a [Uuid],
        db: &'a PgPool,
    ) -> Pin<Box<dyn Future<Output = Result<(), sqlx::Error>> + Send + 'a>>;

    fn update_syncs<'a>(
        ids: &'a [Uuid],
        db: &'a PgPool,
    ) -> Pin<Box<dyn Future<Output = Result<(), sqlx::Error>> + Send + 'a>>;
}

pub async fn connect(config: &Settings) -> Result<sqlx::PgPool, sqlx::Error> {
    let url = format!(
        "postgres://{}:{}@{}:{}/{}",
        config.db.user, config.db.password, config.db.host, config.db.port, config.db.name
    );
    PgPool::connect(&url).await
}

#[derive(FromRow)]
pub struct Data<T: Indexable> {
    pub items: Option<Json<Vec<T>>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Rating {
    pub count: Option<u32>,
    pub value: Option<f64>,
}
