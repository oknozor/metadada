use std::time::Duration;

use mbmeta_db::{Indexable, album::Album, artist::Artist};
use meilisearch_sdk::{client::Client, errors::Error, task_info::TaskInfo};

#[derive(Clone)]
pub struct MeiliClient {
    pub client: Client,
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

    pub async fn setup_indexes(&self) -> Result<(), Error> {
        let artists = self.client.index(Artist::INDEX);
        artists.set_filterable_attributes(["id", "oldids"]).await?;
        artists
            .set_searchable_attributes(["artistname", "sortname"])
            .await?;

        let albums = self.client.index(Album::INDEX);
        albums.set_filterable_attributes(["id", "oldids"]).await?;
        albums
            .set_searchable_attributes(["title", "aliases"])
            .await?;
        Ok(())
    }

    pub async fn add_item<T>(&self, documents: &[T]) -> Result<TaskInfo, Error>
    where
        T: Indexable,
    {
        self.client
            .index(T::INDEX)
            .add_documents(documents, Some(T::ID))
            .await
    }

    pub async fn wait_for_task(&self, task: TaskInfo) -> Result<Status, Error> {
        let task = self
            .client
            .wait_for_task(task, None, Some(Duration::from_secs(360)))
            .await?;

        let task = task
            .wait_for_completion(&self.client, None, Some(Duration::from_secs(60)))
            .await?;

        if task.is_failure() {
            Ok(Status::Failure)
        } else if task.is_success() {
            Ok(Status::Success)
        } else {
            unreachable!("unexpected task status")
        }
    }
}
