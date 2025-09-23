use std::{
    fs::File,
    io::{BufWriter, Write},
};

use crate::{MbLight, error::ReplicationError};
use anyhow::Result;
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::StatusCode;

pub const MUSICBRAINZ_FTP: &str = "http://ftp.musicbrainz.org/pub/musicbrainz/data/fullexport";

impl MbLight {
    pub async fn get_latest(&self) -> Result<String> {
        Ok(self
            .client
            .get(format!("{}/LATEST", MUSICBRAINZ_FTP))
            .send()
            .await?
            .text()
            .await?
            .trim()
            .to_string())
    }

    pub async fn download_with_progress(
        &self,
        url: &str,
        tmpfile: &mut File,
    ) -> Result<(), ReplicationError> {
        let response = self.client.get(url).send().await?;

        if response.status() == StatusCode::NOT_FOUND {
            return Err(ReplicationError::NotFound);
        }

        let total_size = response.content_length().unwrap_or(0);

        let pb = {
            let pb = ProgressBar::new(total_size);
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("{msg}\n - [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                    .unwrap()
                    .progress_chars("=>-"),
            );
            pb.set_message(format!("Downloading {}", url));
            pb
        };

        let mut writer = BufWriter::with_capacity(8 * 1024 * 1024, tmpfile);
        let mut stream = response.bytes_stream();
        let mut buffered_progress: u64 = 0;
        let update_interval: u64 = 256 * 1024;

        while let Some(chunk) = stream.next().await {
            let data = chunk?;
            writer.write_all(&data)?;
            {
                buffered_progress += data.len() as u64;
                if buffered_progress >= update_interval {
                    pb.inc(buffered_progress);
                    buffered_progress = 0;
                }
            }
        }

        {
            if buffered_progress > 0 {
                pb.inc(buffered_progress);
            }

            pb.finish_with_message(format!("Downloaded {}", url));
        }
        Ok(())
    }
}
