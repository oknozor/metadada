use mbmeta_settings::Settings;
use sqlx::PgPool;

pub mod artist;

pub async fn connect(config: &Settings) -> Result<sqlx::PgPool, sqlx::Error> {
    let url = format!(
        "postgres://{}:{}@{}:{}/{}",
        config.db.user, config.db.password, config.db.host, config.db.port, config.db.name
    );
    PgPool::connect(&url).await
}
