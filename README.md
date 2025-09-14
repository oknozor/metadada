 # Medadada

Metadada is an alternaive Lidarr metadata api based on meilisearch search engine.

___

## Important Notes

:warning: **Metadada is not stable yet and may not be a fit for your use case, proceed at your own risk.**

### Initialization and indexing

Note that indexing time may vary depending on your hardware.
The whole initialization took about 2h hour on my 64G RAM/12 cores desktop.
Fortunately many things can still be improved and we expect to reduce it further.

Also be aware that automatic update using the [MusicBrainz Live Data Feed](https://musicbrainz.org/doc/Live_Data_Feed) is not working yet. I should be there soon.

## Local installation

### Prerequisites
- rust/cargo
- docker
- pipx

1. Get meilisearch and postgresql up and running.
  ```sh
  docker compose up -d
  ```

2. Getting a local copy of the musicbrainz database.
  - Install `mbslave`:

     Note that until [this](https://github.com/acoustid/mbslave/pull/23) gets merged, you need to use this fork: https://github.com/oknozor/mbslave

      ```bash
      git clone https://github.com/oknozor/mbslave.git
      cd mbslave
      pipx install .
      ```

  - Configure `mbslave` to use the local database:
      ```ini
      [database]
      host=127.0.0.1
      port=5432
      name=musicbrainz
      user=musicbrainz
      password=musicbrainz
      admin_user=musicbrainz
      admin_password=musicbrainz

      [musicbrainz]
      base_url=https://metabrainz.org/api/musicbrainz/
      token=YourMusicBrainzAPIToken

      [tables]

      [schemas]
      musicbrainz=public
      statistics=statistics
      cover_art_archive=cover_art_archive
      event_art_archive=event_art_archive
      wikidocs=wikidocs
      documentation=documentation
      dbmirror2=dbmirror2
      ignore=
      ```

  - Run `mbslave` to populate the database:

      Note that this may take a while (about 1 hour on my 64G RAM/12 cores desktop)
      ```sh
      mbslave --config=mbslave.ini init
      ```
3. Index the musicbrainz database into the meilisearch instance:

      Expect at least one 1 hour of indexation
      ```sh
     cargo run --release -- init
      ```
4. Run the server:
      ```sh
     cargo run --release -- serve
      ```
5. You can now browse the avalaible entpoint at `localhost:3000/swagger-ui`

## Todos
- [ ] mbslave optimization (ignore unused tables)
- [ ] pg_notify listener for reindexing
- [x] preindexing image and link transformation
- [ ] implement `includeTracks` param using meilisearch `attributesToRetrieve`
- [ ] setup ranking on meilisearch indexes
- [ ] implement /recent endpoints
- [ ] Distribute docker multiarch image
- [ ] Helm charts

## Licence

All the code in this repository is released under the GNU General Public License, for more information take a look at
the [LICENSE](LICENSE) file.
