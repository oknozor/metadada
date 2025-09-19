use chrono::{DateTime, Utc};
use sqlx::{PgPool, prelude::FromRow};

#[derive(Debug, FromRow)]
pub struct ReplicationControl {
    pub current_schema_sequence: Option<i32>,
    pub current_replication_sequence: Option<i32>,
    pub last_replication_date: Option<DateTime<Utc>>,
}

impl ReplicationControl {
    pub async fn get(db: &PgPool) -> Result<Self, sqlx::Error> {
        sqlx::query_as!(ReplicationControl, "SELECT current_schema_sequence, current_replication_sequence, last_replication_date FROM replication_control")
            .fetch_one(db)
            .await
    }

    pub fn next_replication_sequence(&self) -> Option<i32> {
        self.current_replication_sequence.map(|seq| seq + 1)
    }

    pub fn schema_sequence_match(&self, schema_sequence: i32) -> bool {
        self.current_schema_sequence == Some(schema_sequence)
    }
}
