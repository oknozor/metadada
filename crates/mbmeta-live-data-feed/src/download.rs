use anyhow::Result;
use futures_util::StreamExt;
use tokio::io::{AsyncWriteExt, BufWriter};

const MUSICBRAINZ_FTP: &str = "http://ftp.musicbrainz.org/pub/musicbrainz/data/replication";

pub struct ReplicationPacketFetcher {
    client: reqwest::Client,
}

impl ReplicationPacketFetcher {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    pub async fn fetch_packet(
        &self,
        replication_sequence: i32,
        tmpfile: &mut tokio::fs::File,
    ) -> Result<()> {
        let url = format!(
            "{}/replication-{}-v2.tar.bz2",
            MUSICBRAINZ_FTP, replication_sequence
        );

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
            writer.write_all(&data).await?;
        }

        Ok(())
    }
}
