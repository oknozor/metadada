// use anyhow::Result;
// use async_compression::codecs::BzDecoder;
// use async_tar::Archive;
// use chrono::{DateTime, Utc};
// use mbmeta_settings::Settings;
// use sqlx::prelude::FromRow;
// use sqlx::{PgPool, Postgres, Transaction};
// use std::collections::HashMap;
// use std::io::Cursor;

// pub mod async_adapter;
// pub mod dbmirror;
// pub mod download;
// pub mod replication_packet;
// pub mod worker;

// async fn musicbrainz_feed(config: &Settings) -> Result<()> {
//     let pool = PgPool::connect("postgres://musicbrainz:password@localhost/musicbrainz").await?;

//     let pending_keys_map = load_pending_keys(&pool).await?;

//     let xids: Vec<i64> =
//         sqlx::query_scalar!("SELECT DISTINCT xid FROM dbmirror2.pending_data ORDER BY xid ASC")
//             .fetch_all(&pool)
//             .await?;

//     for xid in xids {
//         let mut tx = pool.begin().await?;

//         let tables: Vec<String> = sqlx::query_scalar!(
//             "SELECT DISTINCT tablename FROM dbmirror2.pending_data WHERE xid = $1",
//             xid
//         )
//         .fetch_all(&mut tx)
//         .await?;

//         for table in tables {
//             if let Some(keys) = pending_keys_map.get(&table) {
//                 process_table_by_xid(&mut tx, &table, keys, xid).await?;
//             }
//         }

//         tx.commit().await?;
//     }

//     sqlx::query!("TRUNCATE dbmirror2.pending_data, dbmirror2.pending_keys")
//         .execute(&pool)
//         .await?;

//     println!("MusicBrainz live feed sync completed.");
//     Ok(())
// }

// async fn load_pending_keys(pool: &PgPool) -> Result<HashMap<String, Vec<String>>> {
//     let rows = sqlx::query!("SELECT tablename, keys FROM dbmirror2.pending_keys")
//         .fetch_all(pool)
//         .await?;

//     let mut map = HashMap::new();
//     for row in rows {
//         let key_string = row.keys.trim_matches(|c| c == '{' || c == '}');
//         let key_cols: Vec<String> = key_string
//             .split(',')
//             .map(|s| s.trim().to_string())
//             .collect();
//         map.insert(row.tablename.clone(), key_cols);
//     }
//     Ok(map)
// }

// async fn process_table_by_xid(
//     tx: &mut Transaction<'_, Postgres>,
//     table: &str,
//     keys: &[String],
//     xid: i64,
// ) -> Result<()> {
//     let key_condition = keys
//         .iter()
//         .map(|k| format!("t.{k} = (pd.olddata->>'{k}')::int")) // change to ::uuid if needed
//         .collect::<Vec<_>>()
//         .join(" AND ");

//     let delete_sql = format!(
//         "DELETE FROM {table} t
//          USING dbmirror2.pending_data pd
//          WHERE pd.xid = {xid} AND pd.tablename = '{table}' AND pd.op = 'd'
//            AND {key_condition}"
//     );
//     sqlx::query(&delete_sql).execute(&mut *tx).await?;

//     let update_sql = format!(
//         "UPDATE {table} t
//          SET {table}_json = pd.newdata
//          FROM dbmirror2.pending_data pd
//          WHERE pd.xid = {xid} AND pd.tablename = '{table}' AND pd.op = 'u'
//            AND {key_condition}"
//     );
//     sqlx::query(&update_sql).execute(&mut *tx).await?;

//     let insert_sql = format!(
//         "INSERT INTO {table}
//          SELECT pd.newdata
//          FROM dbmirror2.pending_data pd
//          WHERE pd.xid = {xid} AND pd.tablename = '{table}' AND pd.op = 'i'"
//     );
//     sqlx::query(&insert_sql).execute(&mut *tx).await?;

//     Ok(())
// }
