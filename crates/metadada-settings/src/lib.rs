use std::path::PathBuf;

use config::{Config, Environment, File};
use musicbrainz_light::settings::MbLightSettingsExt;
use once_cell::sync::Lazy;
use serde::Deserialize;

pub static ARTIST_BATCH_SIZE: Lazy<i64> =
    Lazy::new(|| Settings::get().unwrap().sync.artist_batch_size);

pub static ALBUM_BATCH_SIZE: Lazy<i64> =
    Lazy::new(|| Settings::get().unwrap().sync.album_batch_size);

#[derive(Debug, Deserialize, Default, Clone)]
pub struct Settings {
    pub db: DBSettings,
    pub meili: MeiliSettings,
    pub api: ApiSettings,
    pub sync: SyncSettings,
    pub musicbrainz: MusicbrainzSettings,
    pub tables: TableSettings,
    pub schema: SchemaSettings,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct SyncSettings {
    pub artist_batch_size: i64,
    pub album_batch_size: i64,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct ApiSettings {
    pub port: u16,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct MeiliSettings {
    pub url: String,
    pub api_key: String,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct DBSettings {
    pub user: String,
    pub password: String,
    pub host: String,
    pub port: u16,
    pub name: String,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct MusicbrainzSettings {
    pub url: String,
    pub token: String,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct TableSettings {
    keep_only: Vec<String>,
}

impl TableSettings {
    pub fn should_skip(&self, table: &str) -> bool {
        !self.keep_only.is_empty() && !self.keep_only.contains(&table.to_string())
    }
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct SchemaSettings {
    keep_only: Vec<String>,
}

impl SchemaSettings {
    pub fn should_skip(&self, schema: &str) -> bool {
        !self.keep_only.is_empty() && !self.keep_only.contains(&schema.to_string())
    }
}

impl Settings {
    pub fn get() -> Result<Self, config::ConfigError> {
        let mut config = Config::builder().add_source(
            Environment::with_prefix("metadada")
                .try_parsing(true)
                .prefix_separator("__")
                .separator("__"),
        );

        let etc_config = PathBuf::from("/etc/metadada/config.toml");
        if etc_config.exists() {
            config = config.add_source(File::from(etc_config));
        }

        let default_config = PathBuf::from("config.toml");
        if default_config.exists() {
            config = config.add_source(File::from(default_config));
        }

        config.build()?.try_deserialize()
    }

    pub fn db_url(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.db.user, self.db.password, self.db.host, self.db.port, self.db.name
        )
    }
}

impl MbLightSettingsExt for Settings {
    fn db_user(&self) -> &str {
        &self.db.user
    }

    fn db_password(&self) -> &str {
        &self.db.password
    }

    fn db_host(&self) -> &str {
        &self.db.host
    }

    fn db_port(&self) -> u16 {
        self.db.port
    }

    fn db_name(&self) -> &str {
        &self.db.name
    }

    fn table_keep_only(&self) -> &Vec<String> {
        &self.tables.keep_only
    }

    fn schema_keep_only(&self) -> &Vec<String> {
        &self.schema.keep_only
    }

    fn musicbrainz_url(&self) -> &str {
        &self.musicbrainz.url
    }

    fn musicbrainz_token(&self) -> &str {
        &self.musicbrainz.token
    }

    fn should_skip_table(&self, table: &str) -> bool {
        self.tables.should_skip(table)
    }

    fn should_skip_schema(&self, schema: &str) -> bool {
        self.schema.should_skip(schema)
    }
}
