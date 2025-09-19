use std::io::{BufWriter, Write};

use anyhow::Result;
use futures_util::StreamExt;
use tracing::info;

pub struct ReplicationPacketFetcher {
    client: reqwest::Client,
    url: String,
    token: String,
}

impl ReplicationPacketFetcher {
    pub fn new(url: String, token: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            url,
            token,
        }
    }

    pub async fn fetch_packet(
        &self,
        replication_sequence: i32,
        tmpfile: &mut std::fs::File,
    ) -> Result<()> {
        let url = format!(
            "{}/replication-{}-v2.tar.bz2?token={}",
            self.url, replication_sequence, self.token
        );
        info!("Fetching replication packet from {}", url);

        let response = self.client.get(&url).send().await?;
        if response.status() != reqwest::StatusCode::OK {
            return Err(anyhow::anyhow!(
                "Failed to fetch replication-{}-v2.tar.bz2",
                replication_sequence
            ));
        }

        let mut writer = BufWriter::with_capacity(8 * 1024 * 1024, tmpfile);
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let data = chunk?;
            writer.write_all(&data)?;
        }

        Ok(())
    }
}
