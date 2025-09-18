use sqlx::{PgPool, postgres::PgPoolCopyExt};
use tracing::info;

pub async fn load_pending_data(
    db: &PgPool,
    mut source: impl futures_util::io::AsyncReadExt + Unpin,
) -> anyhow::Result<()> {
    let mut copy = db
        .copy_in_raw(
            r#"
            COPY dbmirror2.pending_data FROM STDIN
        "#,
        )
        .await?;

    let mut buf = Vec::new();
    source.read_to_end(&mut buf).await?;
    copy.send(buf.as_slice()).await?;

    let rows = copy.finish().await?;
    info!("Copied {} rows into dbmirror2.pending_data", rows);

    Ok(())
}

pub async fn load_pending_keys(
    db: &PgPool,
    mut source: impl futures_util::io::AsyncReadExt + Unpin,
) -> anyhow::Result<()> {
    let mut copy = db
        .copy_in_raw(
            r#"
            COPY dbmirror2.pending_keys FROM STDIN
        "#,
        )
        .await?;

    let mut buf = Vec::new();
    source.read_to_end(&mut buf).await?;
    copy.send(buf.as_slice()).await?;

    let rows = copy.finish().await?;
    info!("Copied {} rows into dbmirror2.pending_data", rows);

    Ok(())
}

pub async fn truncate_tables(db: &PgPool) -> anyhow::Result<()> {
    sqlx::query!("TRUNCATE dbmirror2.pending_data, dbmirror2.pending_keys")
        .execute(db)
        .await?;

    Ok(())
}
