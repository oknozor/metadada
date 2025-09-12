ALTER TABLE artists_sync
    ADD COLUMN updated_at TIMESTAMPTZ DEFAULT now() NOT NULL;

ALTER TABLE releases_sync
    ADD COLUMN updated_at TIMESTAMPTZ DEFAULT now() NOT NULL;

CREATE OR REPLACE FUNCTION set_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER set_updated_at_artists
BEFORE UPDATE ON artists_sync
FOR EACH ROW
EXECUTE FUNCTION set_updated_at();

CREATE TRIGGER set_updated_at_releases
BEFORE UPDATE ON releases_sync
FOR EACH ROW
EXECUTE FUNCTION set_updated_at();
