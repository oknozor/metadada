use std::path::PathBuf;

use config::{Config, Environment, File};
use once_cell::sync::Lazy;
use serde::Deserialize;

pub static ARTIST_BATCH_SIZE: Lazy<i64> =
    Lazy::new(|| Settings::get().unwrap().sync.artist_batch_size);

pub static ALBUM_BATCH_SIZE: Lazy<i64> =
    Lazy::new(|| Settings::get().unwrap().sync.album_batch_size);

#[derive(Debug, Deserialize, Default)]
pub struct Settings {
    pub db: DBSettings,
    pub meili: MeiliSettings,
    pub api: ApiSettings,
    pub sync: SyncSettings,
}

#[derive(Debug, Deserialize, Default)]
pub struct SyncSettings {
    pub artist_batch_size: i64,
    pub album_batch_size: i64,
}

#[derive(Debug, Deserialize, Default)]
pub struct ApiSettings {
    pub port: u16,
}

#[derive(Debug, Deserialize, Default)]
pub struct MeiliSettings {
    pub url: String,
    pub api_key: String,
}

#[derive(Debug, Deserialize, Default)]
pub struct DBSettings {
    pub user: String,
    pub password: String,
    pub host: String,
    pub port: u16,
    pub name: String,
}

impl Settings {
    pub fn get() -> Result<Self, config::ConfigError> {
        let mut config = Config::builder().add_source(
            Environment::with_prefix("MBMETA")
                .try_parsing(true)
                .prefix_separator("__")
                .separator("__"),
        );

        let etc_config = PathBuf::from("/etc/mbmeta/config.toml");
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
