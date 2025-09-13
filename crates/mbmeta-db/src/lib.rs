use mbmeta_settings::Settings;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, prelude::FromRow, types::Json};

use crate::queryables::QueryAble;

pub mod indexables;
pub mod queryables;

pub async fn connect(config: &Settings) -> Result<sqlx::PgPool, sqlx::Error> {
    let url = format!(
        "postgres://{}:{}@{}:{}/{}",
        config.db.user, config.db.password, config.db.host, config.db.port, config.db.name
    );
    PgPool::connect(&url).await
}

#[derive(FromRow)]
pub struct Data<T: QueryAble> {
    pub items: Option<Json<Vec<T>>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Rating {
    pub count: Option<u32>,
    pub value: Option<f64>,
}
