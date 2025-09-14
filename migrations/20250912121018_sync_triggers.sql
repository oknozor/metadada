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

CREATE OR REPLACE TRIGGER trg_replication_finished
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
-- =====================================

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
    PERFORM flag_entity_unsynced('artist', gid)
    FROM (
        SELECT DISTINCT a.gid
        FROM artist a
        JOIN artist_gid_redirect agr
          ON agr.new_id = COALESCE(NEW.new_id, OLD.new_id)
    ) AS affected;
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
    PERFORM flag_entity_unsynced('artist', gid)
    FROM (
        SELECT DISTINCT a.gid
        FROM artist a
        WHERE a.id = COALESCE(NEW.artist, OLD.artist)
    ) AS affected;
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
    PERFORM flag_entity_unsynced('artist', gid)
    FROM (
        SELECT DISTINCT a.gid
        FROM artist a
        WHERE a.id = COALESCE(NEW.id, OLD.id)
    ) AS affected;
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
    PERFORM flag_entity_unsynced('artist', gid)
    FROM (
        SELECT DISTINCT a.gid
        FROM artist a
        WHERE a.type IN (COALESCE(NEW.id, -1), COALESCE(OLD.id, -1))
    ) AS affected;
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
    PERFORM flag_entity_unsynced('artist', gid)
    FROM (
        SELECT DISTINCT a.gid
        FROM artist a
        JOIN l_artist_url lau
          ON lau.entity0 = a.id
        WHERE lau.entity0 = COALESCE(NEW.entity0, OLD.entity0)
    ) AS affected;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_l_artist_url_changed ON l_artist_url;
CREATE TRIGGER trg_l_artist_url_changed
AFTER INSERT OR UPDATE OR DELETE ON l_artist_url
FOR EACH ROW EXECUTE FUNCTION trg_l_artist_url_changed_fn();

-- url (artist)
CREATE OR REPLACE FUNCTION trg_url_changed_fn()
RETURNS trigger AS $$
BEGIN
    PERFORM flag_entity_unsynced('artist', gid)
    FROM (
        SELECT DISTINCT a.gid
        FROM artist a
        JOIN l_artist_url lau
          ON lau.entity0 = a.id
        WHERE lau.entity1 IN (COALESCE(NEW.id, -1), COALESCE(OLD.id, -1))
    ) AS affected;
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
    PERFORM flag_entity_unsynced('artist', gid)
    FROM (
        SELECT DISTINCT a.gid
        FROM artist a
        WHERE a.id = COALESCE(NEW.artist, OLD.artist)
    ) AS affected;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_artist_tag_changed ON artist_tag;
CREATE TRIGGER trg_artist_tag_changed
AFTER INSERT OR UPDATE OR DELETE ON artist_tag
FOR EACH ROW EXECUTE FUNCTION trg_artist_tag_changed_fn();

-- tag (artist)
CREATE OR REPLACE FUNCTION trg_tag_changed_fn()
RETURNS trigger AS $$
BEGIN
    PERFORM flag_entity_unsynced('artist', gid)
    FROM (
        SELECT DISTINCT a.gid
        FROM artist a
        JOIN artist_tag at ON at.artist = a.id
        WHERE at.tag IN (COALESCE(NEW.id, -1), COALESCE(OLD.id, -1))
    ) AS affected;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_tag_changed ON tag;
CREATE TRIGGER trg_tag_changed
AFTER INSERT OR UPDATE OR DELETE ON tag
FOR EACH ROW EXECUTE FUNCTION trg_tag_changed_fn();

-- genre (artist)
CREATE OR REPLACE FUNCTION trg_genre_changed_fn()
RETURNS trigger AS $$
BEGIN
    PERFORM flag_entity_unsynced('artist', gid)
    FROM (
        SELECT DISTINCT a.gid
        FROM artist a
        JOIN artist_tag at ON at.artist = a.id
        JOIN tag t ON t.id = at.tag
        WHERE t.name IN (COALESCE(NEW.name, ''), COALESCE(OLD.name, ''))
    ) AS affected;
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
        SELECT DISTINCT rg.gid
        FROM release_group rg
        JOIN release_group_gid_redirect rgg
          ON rg.id = rgg.new_id
        WHERE rgg.new_id IN (COALESCE(NEW.new_id, -1), COALESCE(OLD.new_id, -1))
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
        SELECT DISTINCT rg.gid
        FROM release_group rg
        WHERE rg.id = COALESCE(NEW.release_group, OLD.release_group)
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
        SELECT DISTINCT rg.gid
        FROM release_group rg
        WHERE rg.id = COALESCE(NEW.id, OLD.id)
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
        SELECT DISTINCT rg.gid
        FROM release_group rg
        WHERE rg.type IN (COALESCE(NEW.id, -1), COALESCE(OLD.id, -1))
    ) AS affected;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_release_group_primary_type_changed ON release_group_primary_type;
CREATE TRIGGER trg_release_group_primary_type_changed
AFTER INSERT OR UPDATE OR DELETE ON release_group_primary_type
FOR EACH ROW EXECUTE FUNCTION trg_release_group_primary_type_changed_fn();

-- release_group_secondary_type
CREATE OR REPLACE FUNCTION trg_release_group_secondary_type_changed_fn()
RETURNS trigger AS $$
BEGIN
    PERFORM flag_entity_unsynced('release_group', gid)
    FROM (
        SELECT DISTINCT rg.gid
        FROM release_group rg
        JOIN release_group_secondary_type_join rgstj
          ON rg.id = rgstj.release_group
        WHERE rgstj.secondary_type IN (COALESCE(NEW.secondary_type, -1), COALESCE(OLD.secondary_type, -1))
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
        SELECT DISTINCT rg.gid
        FROM release_group rg
        JOIN l_release_group_url lrg
          ON lrg.entity0 = rg.id
        WHERE lrg.entity0 = COALESCE(NEW.entity0, OLD.entity0)
    ) AS affected;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_l_release_group_url_changed ON l_release_group_url;
CREATE TRIGGER trg_l_release_group_url_changed
AFTER INSERT OR UPDATE OR DELETE ON l_release_group_url
FOR EACH ROW EXECUTE FUNCTION trg_l_release_group_url_changed_fn();

-- url (release_group)
CREATE OR REPLACE FUNCTION trg_url_changed_release_group_fn()
RETURNS trigger AS $$
BEGIN
    PERFORM flag_entity_unsynced('release_group', gid)
    FROM (
        SELECT DISTINCT rg.gid
        FROM release_group rg
        JOIN l_release_group_url lrg
          ON lrg.entity0 = rg.id
        WHERE lrg.entity1 IN (COALESCE(NEW.id, -1), COALESCE(OLD.id, -1))
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
        SELECT DISTINCT rg.gid
        FROM release_group rg
        WHERE rg.id = COALESCE(NEW.release_group, OLD.release_group)
    ) AS affected;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_release_group_tag_changed ON release_group_tag;
CREATE TRIGGER trg_release_group_tag_changed
AFTER INSERT OR UPDATE OR DELETE ON release_group_tag
FOR EACH ROW EXECUTE FUNCTION trg_release_group_tag_changed_fn();

-- tag (release_group)
CREATE OR REPLACE FUNCTION trg_tag_changed_release_group_fn()
RETURNS trigger AS $$
BEGIN
    PERFORM flag_entity_unsynced('release_group', gid)
    FROM (
        SELECT DISTINCT rg.gid
        FROM release_group rg
        JOIN release_group_tag rgt
          ON rgt.release_group = rg.id
        WHERE rgt.tag IN (COALESCE(NEW.id, -1), COALESCE(OLD.id, -1))
    ) AS affected;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_tag_changed_release_group ON tag;
CREATE TRIGGER trg_tag_changed_release_group
AFTER INSERT OR UPDATE OR DELETE ON tag
FOR EACH ROW EXECUTE FUNCTION trg_tag_changed_release_group_fn();

-- genre (release_group)
CREATE OR REPLACE FUNCTION trg_genre_changed_release_group_fn()
RETURNS trigger AS $$
BEGIN
    PERFORM flag_entity_unsynced('release_group', gid)
    FROM (
        SELECT DISTINCT rg.gid
        FROM release_group rg
        JOIN release_group_tag rgt
          ON rgt.release_group = rg.id
        JOIN tag t
          ON t.id = rgt.tag
        WHERE t.name IN (COALESCE(NEW.name, ''), COALESCE(OLD.name, ''))
    ) AS affected;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_genre_changed_release_group ON genre;
CREATE TRIGGER trg_genre_changed_release_group
AFTER INSERT OR UPDATE OR DELETE ON genre
FOR EACH ROW EXECUTE FUNCTION trg_genre_changed_release_group_fn();
