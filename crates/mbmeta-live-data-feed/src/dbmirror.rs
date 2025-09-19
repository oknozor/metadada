use std::{collections::BTreeMap, io::Read};

use anyhow::{Context, Result};
use sqlx::{PgPool, Postgres, Transaction, postgres::PgPoolCopyExt, prelude::FromRow};
use tracing::info;

pub async fn load_pending_data(db: &PgPool, mut source: impl Read) -> anyhow::Result<()> {
    let mut copy = db
        .copy_in_raw(
            r#"
            COPY dbmirror2.pending_data FROM STDIN
        "#,
        )
        .await?;

    let mut buf = Vec::new();
    source.read_to_end(&mut buf)?;
    copy.send(buf.as_slice()).await?;

    let rows = copy.finish().await?;
    info!("Copied {} rows into dbmirror2.pending_data", rows);

    Ok(())
}

pub async fn load_pending_keys(db: &PgPool, mut source: impl Read) -> anyhow::Result<()> {
    let mut copy = db
        .copy_in_raw(
            r#"
            COPY dbmirror2.pending_keys FROM STDIN
        "#,
        )
        .await?;

    let mut buf = Vec::new();
    source.read_to_end(&mut buf)?;
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

#[derive(Debug, FromRow)]
pub struct PendingUpdates {
    pub tablename: String,
    pub keys: Vec<String>,
}

impl PendingUpdates {
    async fn all(pool: &PgPool) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            PendingUpdates,
            r#"SELECT tablename, keys FROM dbmirror2.pending_keys"#
        )
        .fetch_all(pool)
        .await
    }
}

pub async fn replicate(db: &PgPool) -> Result<()> {
    info!("Replicating data from dbmirror2.pending_data");
    let pending_keys = PendingUpdates::all(db).await?;
    let mut pending_keys_map = BTreeMap::new();
    for key in pending_keys {
        pending_keys_map.insert(key.tablename, key.keys);
    }

    let xids: Vec<i64> =
        sqlx::query_scalar!("SELECT DISTINCT xid FROM dbmirror2.pending_data ORDER BY xid ASC")
            .fetch_all(db)
            .await?;

    info!("Found {} xids", xids.len());

    for xid in xids {
        let mut tx = db.begin().await?;
        let tables: Vec<String> = sqlx::query_scalar!(
            "SELECT DISTINCT tablename FROM dbmirror2.pending_data WHERE xid = $1",
            xid
        )
        .fetch_all(&mut *tx)
        .await?;

        for table in tables {
            if let Some(keys) = pending_keys_map.get(&table) {
                tx = process_table_by_xid(tx, &table, keys, xid).await?;
            }
        }

        tx.commit().await?;
    }

    sqlx::query!("TRUNCATE dbmirror2.pending_data, dbmirror2.pending_keys")
        .execute(db)
        .await?;

    info!("MusicBrainz live feed sync completed.");
    Ok(())
}

async fn process_table_by_xid<'a>(
    mut tx: Transaction<'a, Postgres>,
    table: &str,
    keys: &[String],
    xid: i64,
) -> Result<Transaction<'a, Postgres>> {
    let (schema, table) = {
        let parts: Vec<_> = table.split('.').collect();
        (parts[0], parts[1])
    };
    let key_condition = keys
        .iter()
        .map(|k| format!("t.{k} = (pd.olddata->>'{k}')::int")) // change to ::uuid if needed
        .collect::<Vec<_>>()
        .join(" AND ");

    let delete_sql = format!(
        r#"DELETE FROM "{schema}"."{table}" t
         USING dbmirror2.pending_data pd
         WHERE pd.xid = {xid} AND pd.tablename = '{table}' AND pd.op = 'd'
           AND {key_condition}"#
    );
    sqlx::query(&delete_sql)
        .execute(&mut *tx)
        .await
        .context("Failed to execute DELETE statement for pending data")?;

    let update_sql = format!(
        r#"INSERT INTO "{schema}"."{table}" (id, name, created, is_active)
                SELECT
                    (pd.newdata->>'id')::uuid,
                    pd.newdata->>'name',
                    (pd.newdata->>'created')::timestamp,
                    (pd.newdata->>'is_active')::boolean
                FROM dbmirror2.pending_data pd
                WHERE pd.xid = $1
                  AND pd.tablename = $2
                  AND pd.op = 'i'"#
    );
    sqlx::query(&update_sql)
        .execute(&mut *tx)
        .await
        .context("Failed to execute UPDATE statement for pending data")?;

    let insert_sql = format!(
        r#"INSERT INTO "{schema}"."{table}" (id, name, created, is_active)
        SELECT
            (pd.newdata->>'id')::uuid,
            pd.newdata->>'name',
            (pd.newdata->>'created')::timestamp,
            (pd.newdata->>'is_active')::boolean
        FROM dbmirror2.pending_data pd
        WHERE pd.xid = $1
          AND pd.tablename = $2
          AND pd.op = 'i';"#
    );
    sqlx::query(&insert_sql)
        .execute(&mut *tx)
        .await
        .context("Failed to execute INSERT statement for pending data")?;

    Ok(tx)
}
