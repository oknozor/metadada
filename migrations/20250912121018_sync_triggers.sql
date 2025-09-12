-- =====================================
-- replication finished triggers
-- ====================================
CREATE OR REPLACE FUNCTION trg_replication_finished_fn()
RETURNS trigger AS $$
BEGIN
  PERFORM pg_notify('replication_finished', NEW.current_replication_sequence::text);
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_replication_finished
AFTER UPDATE ON replication_control
FOR EACH ROW
WHEN (OLD.current_replication_sequence IS DISTINCT FROM NEW.current_replication_sequence)
EXECUTE FUNCTION trg_replication_finished_fn();

-- Flag entity as unsynced when replication finishes
CREATE OR REPLACE FUNCTION flag_entity_unsynced(entity_type text, gid uuid)
RETURNS void AS $$
BEGIN
  IF gid IS NULL THEN
    RETURN;
  END IF;

  IF entity_type = 'artist' THEN
    INSERT INTO artists_sync (id, sync)
    VALUES (gid, FALSE)
    ON CONFLICT (id) DO UPDATE SET sync = FALSE;

  ELSIF entity_type = 'release_group' THEN
    INSERT INTO releases_sync (id, sync)
    VALUES (gid, FALSE)
    ON CONFLICT (id) DO UPDATE SET sync = FALSE;
  END IF;

  PERFORM pg_notify(
    'reindex',
    json_build_object('type', entity_type, 'id', gid)::text
  );
END;
$$ LANGUAGE plpgsql;

-- =====================================
-- artist triggers
-- ====================================
--
-- artist
CREATE OR REPLACE FUNCTION trg_artist_changed_fn()
RETURNS trigger AS $$
BEGIN
  PERFORM flag_entity_unsynced('artist', COALESCE(NEW.gid, OLD.gid));
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_artist_changed ON artist;
CREATE TRIGGER trg_artist_changed
AFTER INSERT OR UPDATE OR DELETE ON artist
FOR EACH ROW EXECUTE FUNCTION trg_artist_changed_fn();

-- artist_gid_redirect
CREATE OR REPLACE FUNCTION trg_artist_gid_redirect_changed_fn()
RETURNS trigger AS $$
BEGIN
  PERFORM flag_entity_unsynced('artist', artist_id)
  FROM (
    SELECT DISTINCT COALESCE(NEW.new_id, OLD.new_id) AS artist_id
  ) AS affected_artists;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_artist_gid_redirect_changed ON artist_gid_redirect;
CREATE TRIGGER trg_artist_gid_redirect_changed
AFTER INSERT OR UPDATE OR DELETE ON artist_gid_redirect
FOR EACH ROW EXECUTE FUNCTION trg_artist_gid_redirect_changed_fn();

-- artist_alias
CREATE OR REPLACE FUNCTION trg_artist_alias_changed_fn()
RETURNS trigger AS $$
BEGIN
  PERFORM flag_entity_unsynced('artist', artist_id)
  FROM (
    SELECT DISTINCT COALESCE(NEW.artist, OLD.artist) AS artist_id
  ) AS affected_artists;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_artist_alias_changed ON artist_alias;
CREATE TRIGGER trg_artist_alias_changed
AFTER INSERT OR UPDATE OR DELETE ON artist_alias
FOR EACH ROW EXECUTE FUNCTION trg_artist_alias_changed_fn();

-- artist_meta
CREATE OR REPLACE FUNCTION trg_artist_meta_changed_fn()
RETURNS trigger AS $$
BEGIN
  PERFORM flag_entity_unsynced('artist', artist_id)
  FROM (
    SELECT DISTINCT COALESCE(NEW.id, OLD.id) AS artist_id
  ) AS affected_artists;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_artist_meta_changed ON artist_meta;
CREATE TRIGGER trg_artist_meta_changed
AFTER INSERT OR UPDATE OR DELETE ON artist_meta
FOR EACH ROW EXECUTE FUNCTION trg_artist_meta_changed_fn();


-- artist_type
CREATE OR REPLACE FUNCTION trg_artist_type_changed_fn()
RETURNS trigger AS $$
BEGIN
  PERFORM flag_entity_unsynced('artist', artist_id)
  FROM (
    SELECT DISTINCT artist.id AS artist_id
    FROM artist
    WHERE artist.type IN (COALESCE(NEW.id, -1), COALESCE(OLD.id, -1))
  ) AS affected_artists;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_artist_type_changed ON artist_type;
CREATE TRIGGER trg_artist_type_changed
AFTER INSERT OR UPDATE OR DELETE ON artist_type
FOR EACH ROW EXECUTE FUNCTION trg_artist_type_changed_fn();

-- artist_url
CREATE OR REPLACE FUNCTION trg_l_artist_url_changed_fn()
RETURNS trigger AS $$
BEGIN
  PERFORM flag_entity_unsynced('artist', artist_id)
  FROM (
    SELECT DISTINCT COALESCE(NEW.entity0, OLD.entity0) AS artist_id
  ) AS affected_artists;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_l_artist_url_changed ON l_artist_url;
CREATE TRIGGER trg_l_artist_url_changed
AFTER INSERT OR UPDATE OR DELETE ON l_artist_url
FOR EACH ROW EXECUTE FUNCTION trg_l_artist_url_changed_fn();

-- url
CREATE OR REPLACE FUNCTION trg_url_changed_fn()
RETURNS trigger AS $$
BEGIN
  PERFORM flag_entity_unsynced('artist', artist_id)
  FROM (
    SELECT DISTINCT l_artist_url.entity0 AS artist_id
    FROM l_artist_url
    WHERE l_artist_url.entity1 IN (COALESCE(NEW.id, -1), COALESCE(OLD.id, -1))
  ) AS affected_artists;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_url_changed ON url;
CREATE TRIGGER trg_url_changed
AFTER INSERT OR UPDATE OR DELETE ON url
FOR EACH ROW EXECUTE FUNCTION trg_url_changed_fn();

-- artist_tag
CREATE OR REPLACE FUNCTION trg_artist_tag_changed_fn()
RETURNS trigger AS $$
BEGIN
  PERFORM flag_entity_unsynced('artist', artist_id)
  FROM (
    SELECT DISTINCT COALESCE(NEW.artist, OLD.artist) AS artist_id
  ) AS affected_artists;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_artist_tag_changed ON artist_tag;
CREATE TRIGGER trg_artist_tag_changed
AFTER INSERT OR UPDATE OR DELETE ON artist_tag
FOR EACH ROW EXECUTE FUNCTION trg_artist_tag_changed_fn();

-- tag
CREATE OR REPLACE FUNCTION trg_tag_changed_fn()
RETURNS trigger AS $$
BEGIN
  PERFORM flag_entity_unsynced('artist', artist_id)
  FROM (
    SELECT DISTINCT artist_tag.artist AS artist_id
    FROM artist_tag
    WHERE artist_tag.tag IN (COALESCE(NEW.id, -1), COALESCE(OLD.id, -1))
  ) AS affected_artists;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_tag_changed ON tag;
CREATE TRIGGER trg_tag_changed
AFTER INSERT OR UPDATE OR DELETE ON tag
FOR EACH ROW EXECUTE FUNCTION trg_tag_changed_fn();

-- genre
CREATE OR REPLACE FUNCTION trg_genre_changed_fn()
RETURNS trigger AS $$
BEGIN
  PERFORM flag_entity_unsynced('artist', artist_id)
  FROM (
    SELECT DISTINCT artist_tag.artist AS artist_id
    FROM artist_tag
    JOIN tag ON tag.id = artist_tag.tag
    WHERE tag.name IN (COALESCE(NEW.name, ''), COALESCE(OLD.name, ''))
  ) AS affected_artists;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_genre_changed ON genre;
CREATE TRIGGER trg_genre_changed
AFTER INSERT OR UPDATE OR DELETE ON genre
FOR EACH ROW EXECUTE FUNCTION trg_genre_changed_fn();

-- =====================================
-- release_group triggers
-- =====================================

-- release_group
CREATE OR REPLACE FUNCTION trg_release_group_changed_fn()
RETURNS trigger AS $$
BEGIN
  PERFORM flag_entity_unsynced('release_group', COALESCE(NEW.gid, OLD.gid));
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_release_group_changed ON release_group;
CREATE TRIGGER trg_release_group_changed
AFTER INSERT OR UPDATE OR DELETE ON release_group
FOR EACH ROW EXECUTE FUNCTION trg_release_group_changed_fn();


-- release_group_gid_redirect
CREATE OR REPLACE FUNCTION trg_release_group_gid_redirect_changed_fn()
RETURNS trigger AS $$
BEGIN
  PERFORM flag_entity_unsynced('release_group', gid)
  FROM (
    SELECT DISTINCT COALESCE(NEW.new_id, OLD.new_id) AS gid
  ) AS affected;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_release_group_gid_redirect_changed ON release_group_gid_redirect;
CREATE TRIGGER trg_release_group_gid_redirect_changed
AFTER INSERT OR UPDATE OR DELETE ON release_group_gid_redirect
FOR EACH ROW EXECUTE FUNCTION trg_release_group_gid_redirect_changed_fn();


-- release_group_alias
CREATE OR REPLACE FUNCTION trg_release_group_alias_changed_fn()
RETURNS trigger AS $$
BEGIN
  PERFORM flag_entity_unsynced('release_group', gid)
  FROM (
    SELECT DISTINCT COALESCE(NEW.release_group, OLD.release_group) AS gid
  ) AS affected;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_release_group_alias_changed ON release_group_alias;
CREATE TRIGGER trg_release_group_alias_changed
AFTER INSERT OR UPDATE OR DELETE ON release_group_alias
FOR EACH ROW EXECUTE FUNCTION trg_release_group_alias_changed_fn();


-- release_group_meta
CREATE OR REPLACE FUNCTION trg_release_group_meta_changed_fn()
RETURNS trigger AS $$
BEGIN
  PERFORM flag_entity_unsynced('release_group', gid)
  FROM (
    SELECT DISTINCT COALESCE(NEW.id, OLD.id) AS gid
  ) AS affected;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_release_group_meta_changed ON release_group_meta;
CREATE TRIGGER trg_release_group_meta_changed
AFTER INSERT OR UPDATE OR DELETE ON release_group_meta
FOR EACH ROW EXECUTE FUNCTION trg_release_group_meta_changed_fn();


-- release_group_primary_type
CREATE OR REPLACE FUNCTION trg_release_group_primary_type_changed_fn()
RETURNS trigger AS $$
BEGIN
  PERFORM flag_entity_unsynced('release_group', gid)
  FROM (
    SELECT DISTINCT release_group.id AS gid
    FROM release_group
    WHERE release_group.type IN (COALESCE(NEW.id, -1), COALESCE(OLD.id, -1))
  ) AS affected;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_release_group_primary_type_changed ON release_group_primary_type;
CREATE TRIGGER trg_release_group_primary_type_changed
AFTER INSERT OR UPDATE OR DELETE ON release_group_primary_type
FOR EACH ROW EXECUTE FUNCTION trg_release_group_primary_type_changed_fn();


-- release_group_secondary_type and join table
CREATE OR REPLACE FUNCTION trg_release_group_secondary_type_changed_fn()
RETURNS trigger AS $$
BEGIN
  PERFORM flag_entity_unsynced('release_group', gid)
  FROM (
    SELECT DISTINCT release_group.id AS gid
    FROM release_group
    JOIN release_group_secondary_type_join rgstj
      ON release_group.id = rgstj.release_group
    WHERE rgstj.secondary_type IN (COALESCE(NEW.id, -1), COALESCE(OLD.id, -1))
  ) AS affected;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_release_group_secondary_type_changed ON release_group_secondary_type;
CREATE TRIGGER trg_release_group_secondary_type_changed
AFTER INSERT OR UPDATE OR DELETE ON release_group_secondary_type
FOR EACH ROW EXECUTE FUNCTION trg_release_group_secondary_type_changed_fn();

DROP TRIGGER IF EXISTS trg_release_group_secondary_type_join_changed ON release_group_secondary_type_join;
CREATE TRIGGER trg_release_group_secondary_type_join_changed
AFTER INSERT OR UPDATE OR DELETE ON release_group_secondary_type_join
FOR EACH ROW EXECUTE FUNCTION trg_release_group_secondary_type_changed_fn();


-- l_release_group_url
CREATE OR REPLACE FUNCTION trg_l_release_group_url_changed_fn()
RETURNS trigger AS $$
BEGIN
  PERFORM flag_entity_unsynced('release_group', gid)
  FROM (
    SELECT DISTINCT COALESCE(NEW.entity0, OLD.entity0) AS gid
  ) AS affected;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_l_release_group_url_changed ON l_release_group_url;
CREATE TRIGGER trg_l_release_group_url_changed
AFTER INSERT OR UPDATE OR DELETE ON l_release_group_url
FOR EACH ROW EXECUTE FUNCTION trg_l_release_group_url_changed_fn();


-- url
CREATE OR REPLACE FUNCTION trg_url_changed_release_group_fn()
RETURNS trigger AS $$
BEGIN
  PERFORM flag_entity_unsynced('release_group', gid)
  FROM (
    SELECT DISTINCT l_release_group_url.entity0 AS gid
    FROM l_release_group_url
    WHERE l_release_group_url.entity1 IN (COALESCE(NEW.id, -1), COALESCE(OLD.id, -1))
  ) AS affected;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_url_changed_release_group ON url;
CREATE TRIGGER trg_url_changed_release_group
AFTER INSERT OR UPDATE OR DELETE ON url
FOR EACH ROW EXECUTE FUNCTION trg_url_changed_release_group_fn();


-- release_group_tag
CREATE OR REPLACE FUNCTION trg_release_group_tag_changed_fn()
RETURNS trigger AS $$
BEGIN
  PERFORM flag_entity_unsynced('release_group', gid)
  FROM (
    SELECT DISTINCT COALESCE(NEW.release_group, OLD.release_group) AS gid
  ) AS affected;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_release_group_tag_changed ON release_group_tag;
CREATE TRIGGER trg_release_group_tag_changed
AFTER INSERT OR UPDATE OR DELETE ON release_group_tag
FOR EACH ROW EXECUTE FUNCTION trg_release_group_tag_changed_fn();


-- tag
CREATE OR REPLACE FUNCTION trg_tag_changed_release_group_fn()
RETURNS trigger AS $$
BEGIN
  PERFORM flag_entity_unsynced('release_group', gid)
  FROM (
    SELECT DISTINCT release_group_tag.release_group AS gid
    FROM release_group_tag
    WHERE release_group_tag.tag IN (COALESCE(NEW.id, -1), COALESCE(OLD.id, -1))
  ) AS affected;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_tag_changed_release_group ON tag;
CREATE TRIGGER trg_tag_changed_release_group
AFTER INSERT OR UPDATE OR DELETE ON tag
FOR EACH ROW EXECUTE FUNCTION trg_tag_changed_release_group_fn();


-- genre
CREATE OR REPLACE FUNCTION trg_genre_changed_release_group_fn()
RETURNS trigger AS $$
BEGIN
  PERFORM flag_entity_unsynced('release_group', gid)
  FROM (
    SELECT DISTINCT release_group_tag.release_group AS gid
    FROM release_group_tag
    JOIN tag ON tag.id = release_group_tag.tag
    WHERE tag.name IN (COALESCE(NEW.name, ''), COALESCE(OLD.name, ''))
  ) AS affected;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_genre_changed_release_group ON genre;
CREATE TRIGGER trg_genre_changed_release_group
AFTER INSERT OR UPDATE OR DELETE ON genre
FOR EACH ROW EXECUTE FUNCTION trg_genre_changed_release_group_fn();
