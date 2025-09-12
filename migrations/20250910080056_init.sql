-- Add migration script here
CREATE TABLE artists_sync (
    id uuid PRIMARY KEY,
    sync BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE TABLE releases_sync (
    id uuid PRIMARY KEY,
    sync BOOLEAN NOT NULL DEFAULT FALSE
);
