use std::sync::Arc;

use metadada_settings::Settings;
use sqlx::PgPool;
use tokio::sync::mpsc::Sender;
use tracing::info;

pub mod download;
mod error;
mod musicbrainz_db;
mod tar_helper;

pub struct MbLight {
    pub client: reqwest::Client,
    pub config: Arc<Settings>,
    pub db: PgPool,
    pub reindex_sender: Sender<()>,
}

impl MbLight {
    pub fn new(config: Settings, db: PgPool, reindex_sender: Sender<()>) -> Self {
        Self {
            client: reqwest::Client::new(),
            config: Arc::new(config),
            db,
            reindex_sender,
        }
    }

    async fn should_skip_table(&self, schema: &str, table: &str) -> anyhow::Result<bool> {
        if self.config.schema.should_skip(schema) {
            return Ok(true);
        }

        if self.config.tables.should_skip(table) {
            return Ok(true);
        }
        let fulltable = format!("{}.{}", schema, table);

        let table_exists: bool = sqlx::query_scalar!(
            "SELECT EXISTS (
                 SELECT FROM information_schema.tables
                 WHERE table_schema = $1 AND table_name = $2
             )",
            schema,
            table
        )
        .fetch_one(&self.db)
        .await
        .map(Option::unwrap_or_default)?;

        if !table_exists {
            info!("Skipping {} (table {} does not exist)", table, fulltable);
            return Ok(true);
        }

        let has_data: bool = self.has_data(schema, table).await?;

        if has_data {
            info!(
                "Skipping {} (table {} already contains data)",
                table, fulltable
            );
            return Ok(true);
        }

        Ok(false)
    }

    pub async fn has_data(&self, schema: &str, table: &str) -> anyhow::Result<bool> {
        let fulltable = format!("{}.{}", schema, table);

        let has_data: bool = sqlx::query_scalar(&format!(
            "SELECT EXISTS (SELECT 1 FROM {} LIMIT 1)",
            fulltable
        ))
        .fetch_one(&self.db)
        .await
        .map(Option::unwrap_or_default)?;

        Ok(has_data)
    }
}
