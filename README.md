 # Metadada

Metadada is an alternaive Lidarr metadata api based on MusicBrainz database and Meilisearch.


### Important Notes

:warning: **Metadada is not stable yet and may not be a fit for your use case, proceed at your own risk.**

## Initialization and indexing

Note that indexing time may vary depending on your hardware.
The whole initialization took about 2h hour on my 64G RAM/12 cores desktop.
Fortunately many things can still be improved and we expect to reduce it further.

## Running locally

### Prerequisites
- rust/cargo
- docker

1. Get meilisearch and postgresql up and running.
  ```sh
  docker compose up -d
  ```

2. Setting up Metadada config file:
  ```sh
  cp config.example.toml config.toml
  ```

  If running locally, the only thing you need to change is the Musicbrainz token (which can be obtained following [the official documentation](https://musicbrainz.org/doc/Development/OAuth2)).

3. Install Metadada
  ```sh
  cargo install --path .
  ```

4. Run Metadada
  ```sh
  metadada
  ```

  **Important:** Prior to starting the API Metadada will check for existing entries in the database, if none are found it will start replicating the MusicBrainz database, Note that this may take a while depending on your hardware.

5. Once the API is up and running, you can browse the avalaible entpoint at `localhost:3000/swagger-ui`

6. Changing the Lidarr metadata server (adapt the url to your needs)
```sql
INSERT INTO Config (Key, Value)
        VALUES ('metadatasource', 'http://localhost:3000')
        ON CONFLICT(Key) DO UPDATE SET Value = excluded.Value
```

## Licence

All the code in this repository is released under the GNU General Public License, for more information take a look at
the [LICENSE](LICENSE) file.
