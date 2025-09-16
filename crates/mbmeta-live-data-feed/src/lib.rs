fn main() {}
// use anyhow::Result;
// use sqlx::{PgPool, Postgres, Transaction};
// use std::collections::HashMap;

// #[tokio::main]
// async fn main() -> Result<()> {
//     let pool = PgPool::connect("postgres://musicbrainz:password@localhost/musicbrainz").await?;

//     // 1️⃣ Load table keys
//     let pending_keys_map = load_pending_keys(&pool).await?;

//     // 2️⃣ Get all XIDs to process, in order
//     let xids: Vec<i64> = sqlx::query_scalar!(
//         "SELECT DISTINCT xid FROM dbmirror2.pending_data ORDER BY xid ASC"
//     )
//     .fetch_all(&pool)
//     .await?;

//     for xid in xids {
//         let mut tx = pool.begin().await?;

//         // Tables in this XID
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

//     // Clean up
//     sqlx::query!("TRUNCATE dbmirror2.pending_data, dbmirror2.pending_keys")
//         .execute(&pool)
//         .await?;

//     println!("MusicBrainz live feed sync completed.");
//     Ok(())
// }

// /// Load pending_keys into a HashMap: table_name -> Vec<key_columns>
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

// /// Process DELETE, UPDATE, INSERT per table for a given XID
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

//     // DELETE
//     let delete_sql = format!(
//         "DELETE FROM {table} t
//          USING dbmirror2.pending_data pd
//          WHERE pd.xid = {xid} AND pd.tablename = '{table}' AND pd.op = 'd'
//            AND {key_condition}"
//     );
//     sqlx::query(&delete_sql).execute(&mut *tx).await?;

//     // UPDATE
//     let update_sql = format!(
//         "UPDATE {table} t
//          SET {table}_json = pd.newdata
//          FROM dbmirror2.pending_data pd
//          WHERE pd.xid = {xid} AND pd.tablename = '{table}' AND pd.op = 'u'
//            AND {key_condition}"
//     );
//     sqlx::query(&update_sql).execute(&mut *tx).await?;

//     // INSERT
//     let insert_sql = format!(
//         "INSERT INTO {table}
//          SELECT pd.newdata
//          FROM dbmirror2.pending_data pd
//          WHERE pd.xid = {xid} AND pd.tablename = '{table}' AND pd.op = 'i'"
//     );
//     sqlx::query(&insert_sql).execute(&mut *tx).await?;

//     Ok(())
// }

// use anyhow::Result;
// use bzip2::read::BzDecoder;
// use reqwest::blocking::Client;
// use std::fs::File;
// use std::io::{self, Cursor, Read};
// use tar::Archive;

// /// Download the replication packet and return a reader to its contents
// fn download_packet(base_url: &str, token: Option<&str>, replication_seq: u32) -> Result<Cursor<Vec<u8>>> {
//     let mut url = format!("{}/replication-{}-v2.tar.bz2", base_url.trim_end_matches('/'), replication_seq);
//     if let Some(token) = token {
//         url.push_str(&format!("?token={}", token));
//     }

//     println!("Downloading {}", url);

//     let client = Client::builder().timeout(std::time::Duration::from_secs(60)).build()?;
//     let mut resp = client.get(&url).send()?;

//     if resp.status().as_u16() == 404 {
//         anyhow::bail!("Packet {} not found", replication_seq);
//     }

//     let mut data = Vec::new();
//     resp.copy_to(&mut data)?;
//     Ok(Cursor::new(data))
// }

// /// Extract a .tar.bz2 archive into the given output directory
// fn extract_packet(reader: Cursor<Vec<u8>>, output_dir: &str) -> Result<()> {
//     let bz = BzDecoder::new(reader);
//     let mut archive = Archive::new(bz);
//     archive.unpack(output_dir)?;
//     Ok(())
// }
