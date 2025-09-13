use std::{fmt::Debug, pin::Pin};

use serde::{Serialize, de::DeserializeOwned};
use sqlx::PgPool;
use uuid::Uuid;

pub mod album;
pub mod artist;

pub trait QueryAble: DeserializeOwned + Send + Sync + Debug {
    type Indexable: From<Self> + Send + Sync + Serialize + Debug;
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

    fn to_model(self) -> Self::Indexable;
}
