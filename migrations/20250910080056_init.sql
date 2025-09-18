CREATE SCHEMA metadada;

CREATE TABLE metadada.artists_sync (
    id uuid PRIMARY KEY,
    sync BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE TABLE metadada.releases_sync (
    id uuid PRIMARY KEY,
    sync BOOLEAN NOT NULL DEFAULT FALSE
);
