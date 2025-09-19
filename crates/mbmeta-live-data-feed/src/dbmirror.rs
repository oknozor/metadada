use std::io::Read;

use anyhow::Result;
use serde_json::Value;
use sqlx::{PgPool, Postgres, Transaction, postgres::PgPoolCopyExt, prelude::FromRow};
use tracing::info;

#[derive(Debug, FromRow)]
pub struct Column {
    pub name: Option<String>,
    pub data_type: Option<String>,
}

impl Column {
    pub async fn get(schema: &str, table: &str, db: &PgPool) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            Column,
            r#"
            SELECT column_name as name, data_type
            FROM information_schema.columns
            WHERE table_schema = $1
              AND table_name = $2
              AND is_identity = 'NO'
            "#,
            schema,
            table
        )
        .fetch_all(db)
        .await
    }

    pub fn cast_expr(&self, json_var: &str) -> Option<String> {
        let name = self.name.as_ref()?;
        let data_type = self.data_type.as_ref()?;

        let cast = match data_type.as_str() {
            "smallint" => "::smallint",
            "integer" => "::int",
            "bigint" => "::bigint",
            "boolean" => "::boolean",
            "uuid" => "::uuid",
            "timestamp with time zone" => "::timestamptz",
            "timestamp without time zone" => "::timestamp",
            "USER-DEFINED" if name == "toc" => "::cube",
            "text" | "character varying" => "",
            "USER-DEFINED" if name == "cover_art_presence" => {
                return Some(format!(
                    "(({json_var}->>'{name}')::text)::cover_art_presence"
                ));
            }
            _ => "",
        };

        Some(format!("({json_var}->>'{name}'){cast}"))
    }
}

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
    info!("Copied {} rows into dbmirror2.pending_keys", rows);

    Ok(())
}

pub async fn truncate_tables(db: &PgPool) -> anyhow::Result<()> {
    sqlx::query!("TRUNCATE dbmirror2.pending_data, dbmirror2.pending_keys")
        .execute(db)
        .await?;

    Ok(())
}

#[derive(sqlx::FromRow, Debug)]
struct PendingRow {
    seqid: i64,
    xid: i64,
    op: CharType,
    olddata: Option<Value>,
    newdata: Option<Value>,
    tablename: String,
    keys: Vec<String>,
}

#[derive(Debug)]
struct CharType(i8);

impl From<i8> for CharType {
    fn from(value: i8) -> Self {
        CharType(value)
    }
}

impl CharType {
    fn as_char(&self) -> char {
        self.0 as u8 as char
    }
}

impl PendingRow {
    async fn all(db: &PgPool) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            PendingRow,
            r#"
                SELECT pd.xid,
                       pd.seqid,
                       pd.tablename,
                       pd.op,
                       pk.keys,
                       pd.olddata,
                       pd.newdata
                  FROM dbmirror2.pending_data pd
                  JOIN dbmirror2.pending_keys pk
                    ON pk.tablename = pd.tablename
                     ORDER BY pd.xid, pd.seqid
                "#,
        )
        .fetch_all(db)
        .await
    }
}

pub async fn replicate(db: &PgPool) -> Result<()> {
    info!("Replicating data from dbmirror2.pending_data");
    let mut tx = db.begin().await?;
    for row in PendingRow::all(&db).await? {
        match row.op.as_char() {
            'i' => apply_insert(&mut tx, &row, db).await?,
            'u' => apply_update(&mut tx, &row, db).await?,
            'd' => apply_delete(&mut tx, &row, db).await?,
            _ => unreachable!(),
        }
    }
    tx.commit().await?;
    info!("Replication succedeed, truncating db mirror");
    truncate_tables(db).await?;
    Ok(())
}

async fn apply_delete(
    tx: &mut Transaction<'_, Postgres>,
    row: &PendingRow,
    db: &PgPool,
) -> Result<()> {
    println!("D {:?}", row);
    let (schema, table) = {
        let parts: Vec<_> = row.tablename.split('.').collect();
        (parts[0], parts[1])
    };

    let columns = Column::get(schema, table, db).await?;

    // Build WHERE clause only using the PK columns from pending_keys
    let where_clause: Vec<String> = row
        .keys
        .iter()
        .filter_map(|key| {
            columns
                .iter()
                .find(|c| c.name.as_deref() == Some(key))
                .and_then(|c| c.cast_expr("pd.olddata"))
        })
        .collect();

    if where_clause.is_empty() {
        return Err(anyhow::anyhow!("No matching keys found for DELETE"));
    }

    let sql = format!(
        r#"DELETE FROM "{schema}"."{table}" t
           USING (SELECT $1::jsonb AS olddata) pd
           WHERE {}"#,
        where_clause.join(" AND ")
    );

    sqlx::query(&sql)
        .bind(row.olddata.as_ref().unwrap())
        .execute(&mut **tx)
        .await?;

    Ok(())
}

async fn apply_update(
    tx: &mut Transaction<'_, Postgres>,
    row: &PendingRow,
    db: &PgPool,
) -> Result<()> {
    println!("U {:?}", row);
    let (schema, table) = {
        let parts: Vec<_> = row.tablename.split('.').collect();
        (parts[0], parts[1])
    };

    let columns = Column::get(schema, table, db).await?;

    // SET clause: all non-PK columns
    let set_clause: Vec<String> = columns
        .iter()
        .filter(|c| {
            c.name
                .as_ref()
                .map(|n| !row.keys.contains(n))
                .unwrap_or(false)
        })
        .filter_map(|c| {
            c.cast_expr("pd.newdata")
                .map(|expr| format!("{0} = {1}", c.name.as_ref().unwrap(), expr))
        })
        .collect();

    if set_clause.is_empty() {
        return Ok(()); // nothing to update
    }

    // WHERE clause: only PK columns
    let where_clause: Vec<String> = row
        .keys
        .iter()
        .filter_map(|key| {
            columns
                .iter()
                .find(|c| c.name.as_deref() == Some(key))
                .and_then(|c| {
                    c.cast_expr("pd.olddata")
                        .map(|expr| format!("{0} = {1}", key, expr))
                })
        })
        .collect();

    if where_clause.is_empty() {
        return Err(anyhow::anyhow!("No matching keys found for UPDATE"));
    }

    let sql = format!(
        r#"UPDATE "{schema}"."{table}" t
           SET {}
           FROM (SELECT $1::jsonb AS olddata, $2::jsonb AS newdata) pd
           WHERE {}"#,
        set_clause.join(", "),
        where_clause.join(" AND ")
    );

    sqlx::query(&sql)
        .bind(row.olddata.as_ref().unwrap())
        .bind(row.newdata.as_ref().unwrap())
        .execute(&mut **tx)
        .await?;

    Ok(())
}

async fn apply_insert(
    tx: &mut Transaction<'_, Postgres>,
    row: &PendingRow,
    db: &PgPool,
) -> Result<()> {
    println!("I {:?}", row);
    let (schema, table) = {
        let parts: Vec<_> = row.tablename.split('.').collect();
        (parts[0], parts[1])
    };

    let columns = Column::get(schema, table, db).await?;

    // Column names for insert
    let col_names: Vec<String> = columns.iter().filter_map(|c| c.name.clone()).collect();

    // Values dynamically cast
    let vals: Vec<String> = columns
        .iter()
        .filter_map(|c| c.cast_expr("pd.newdata"))
        .collect();

    let sql = format!(
        r#"INSERT INTO "{schema}"."{table}" ({})
           SELECT {} FROM (SELECT $1::jsonb AS newdata) pd"#,
        col_names.join(", "),
        vals.join(", ")
    );

    println!("{:?}", sql);

    sqlx::query(&sql)
        .bind(row.newdata.as_ref().unwrap())
        .execute(&mut **tx)
        .await?;

    Ok(())
}
