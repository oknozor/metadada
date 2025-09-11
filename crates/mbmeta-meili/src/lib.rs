use std::time::Duration;

use mbmeta_model::Artist;
use meilisearch_sdk::{client::Client, errors::Error, task_info::TaskInfo};

#[derive(Clone)]
pub struct MeiliClient {
    client: Client,
}

pub enum Status {
    Success,
    Failure,
}

impl MeiliClient {
    pub fn new(url: &str, api_key: &str) -> Self {
        let client =
            Client::new(url, Some(api_key)).expect("failed to initialize meilisearch client");
        Self { client }
    }

    pub async fn add_artists(&self, documents: &[Artist]) -> Result<TaskInfo, Error> {
        self.client
            .index("artists")
            .add_documents(documents, Some("id"))
            .await
    }

    pub async fn wait_for_task(&self, task: TaskInfo) -> Result<Status, Error> {
        let task = self.client.wait_for_task(task, None, None).await?;

        let task = task
            .wait_for_completion(&self.client, None, Some(Duration::from_secs(60)))
            .await?;

        if task.is_failure() {
            return Ok(Status::Failure);
        } else if task.is_success() {
            return Ok(Status::Success);
        } else {
            unreachable!("unexpected task status")
        }
    }
}
