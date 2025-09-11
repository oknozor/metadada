1. Add a docker compose file
2. script all the musicbrainz initialization
3. create a crawling pipeline (maybe using quickwit agent)
  - assess meilisearch, index size and indexing speed
4. meilisearch task webhook server
5. spotify + coverart resolver using flawless
  - reassess indexing time and size (maybe introduce an intermediary db to mark indexed items)

- Route
- GET /artist/{mbid}
- GET /album/{mbid}
- GET /search/album
- GET /search/artist
- GET /search/all
- GET /search/fingerprint
