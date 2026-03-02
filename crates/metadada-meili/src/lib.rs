use std::time::Duration;

use meilisearch_sdk::{client::Client, errors::Error, task_info::TaskInfo};
use metadada_db::queryables::{QueryAble, album::Album, artist::Artist};

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

    pub async fn setup_artist_index(&self) -> Result<(), Error> {
        let artists = self.client.index(Artist::INDEX);

        artists
            .set_ranking_rules([
                "words",
                "typo",
                "proximity",
                "attribute",
                "sort",
                "exactness",
                "rating.value:desc",
            ])
            .await?
            .wait_for_completion(&self.client, None, None)
            .await?;

        artists
            .set_sortable_attributes(["rating.value"])
            .await?
            .wait_for_completion(&self.client, None, None)
            .await?;

        artists
            .set_filterable_attributes(["id", "oldids", "genres", "type", "status"])
            .await?
            .wait_for_completion(&self.client, None, None)
            .await?;

        artists
            .set_searchable_attributes(["artistname", "sortname", "artistaliases"])
            .await?
            .wait_for_completion(&self.client, None, None)
            .await?;

        Ok(())
    }

    pub async fn setup_album_index(&self) -> Result<(), Error> {
        let albums = self.client.index(Album::INDEX);

        albums
            .set_ranking_rules([
                "words",
                "typo",
                "proximity",
                "attribute",
                "sort",
                "exactness",
                "rating.value:desc",
                "releasedate:desc",
            ])
            .await?
            .wait_for_completion(&self.client, None, None)
            .await?;

        albums
            .set_sortable_attributes(["rating.value", "releasedate"])
            .await?
            .wait_for_completion(&self.client, None, None)
            .await?;

        albums
            .set_filterable_attributes(["id", "oldids", "genres", "type", "artistid", "artistids"])
            .await?
            .wait_for_completion(&self.client, None, None)
            .await?;

        albums
            .set_searchable_attributes(["title", "aliases", "artists.artistname"])
            .await?
            .wait_for_completion(&self.client, None, None)
            .await?;

        Ok(())
    }

    pub async fn add_item<T>(&self, documents: Vec<T>) -> Result<TaskInfo, Error>
    where
        T: QueryAble,
    {
        let documents = documents
            .into_iter()
            .map(QueryAble::to_model)
            .collect::<Vec<_>>();

        self.client
            .index(T::INDEX)
            .add_documents(&documents, Some(T::ID))
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
