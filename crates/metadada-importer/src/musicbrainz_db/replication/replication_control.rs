use sqlx::{
    PgPool,
    types::chrono::{DateTime, Utc},
};

pub struct ReplicationControl {
    pub current_schema_sequence: Option<i32>,
    pub current_replication_sequence: Option<i32>,
    pub last_replication_date: Option<DateTime<Utc>>,
}

impl ReplicationControl {
    pub async fn get(db: &PgPool) -> Result<Self, sqlx::Error> {
        sqlx::query_as!(
            Self,
            "SELECT current_schema_sequence, current_replication_sequence, last_replication_date FROM replication_control"
        )
        .fetch_one(db)
        .await
    }

    pub async fn update(self, db: &PgPool) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE replication_control SET current_replication_sequence = $1, last_replication_date = $2",
            self.next_replication_sequence(),
            Utc::now()
        )
        .execute(db)
        .await?;
        Ok(())
    }

    pub fn next_replication_sequence(&self) -> Option<i32> {
        self.current_replication_sequence.map(|seq| seq + 1)
    }

    pub fn schema_sequence_match(&self, actual: i32) -> bool {
        self.current_schema_sequence == Some(actual)
    }

    pub fn next_replication_packet_url(&self, base: &str, token: &str) -> Option<String> {
        let seq = self.next_replication_sequence()?;
        Some(format!("{base}/replication-{seq}-v2.tar.bz2?token={token}"))
    }
}
