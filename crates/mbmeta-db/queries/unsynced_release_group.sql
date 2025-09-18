SELECT
  json_agg(album_data) AS "items: Json<Vec<Album>>"
FROM (
  SELECT
    release_group.gid AS Id,
    array(
      SELECT gid
      FROM release_group_gid_redirect
      WHERE release_group_gid_redirect.new_id = release_group.id
    ) AS OldIds,
    release_group.comment AS Disambiguation,
    release_group.name AS Title,
    artist.gid as ArtistId,
    array(
      SELECT DISTINCT artist.gid
      FROM artist
      JOIN artist_credit_name ON artist_credit_name.artist = artist.id
      WHERE artist_credit_name.artist_credit = release_group.artist_credit
        AND artist_credit_name.position = 0
      UNION
      SELECT DISTINCT artist.gid
      FROM artist
      JOIN artist_credit_name ON artist_credit_name.artist = artist.id
      JOIN track ON track.artist_credit = artist_credit_name.artist_credit
      JOIN medium ON track.medium = medium.id
      JOIN release ON medium.release = release.id
      WHERE release.release_group = release_group.id
        AND artist_credit_name.position = 0
    ) AS ArtistIds,
    array(
      SELECT name
      FROM release_group_alias
      WHERE release_group_alias.release_group = release_group.id
        AND (release_group_alias.type IS NULL OR release_group_alias.type = 1)
      UNION
      SELECT release.name
      FROM release
      WHERE release.release_group = release_group.id
        AND release.name != release_group.name
    ) AS Aliases,
    COALESCE(release_group_primary_type.name, 'Other') AS Type,
    array(
      SELECT name
      FROM release_group_secondary_type rgst
      JOIN release_group_secondary_type_join rgstj ON rgstj.secondary_type = rgst.id
      WHERE rgstj.release_group = release_group.id
      ORDER BY name ASC
    ) AS SecondaryTypes,
    COALESCE(
      make_date(
        release_group_meta.first_release_date_year,
        release_group_meta.first_release_date_month,
        release_group_meta.first_release_date_day
      ),
      make_date(
        COALESCE(release_group_meta.first_release_date_year, 1),
        COALESCE(release_group_meta.first_release_date_month, 1),
        COALESCE(release_group_meta.first_release_date_day, 1)
      )
    ) AS ReleaseDate,
    (
      SELECT json_agg(row_to_json(artist_data))
      FROM (
        SELECT
          artist.gid AS Id,
          array(
            SELECT gid
            FROM artist_gid_redirect
            WHERE artist_gid_redirect.new_id = artist.id
          ) AS OldIds,
          artist.name AS ArtistName,
          artist.sort_name AS SortName,
          array(
            SELECT name
            FROM artist_alias
            WHERE artist_alias.artist = artist.id
              AND (artist_alias.type IS NULL OR artist_alias.type = 1)
          ) AS ArtistAliases,
          CASE WHEN artist.ended THEN 'ended' ELSE 'active' END AS Status,
          artist.comment AS Disambiguation,
          artist_type.name AS Type,
          json_build_object(
            'Count', COALESCE(artist_meta.rating_count, 0),
            'Value', artist_meta.rating::decimal / 10
          ) AS Rating,
          array(
            SELECT url.url
            FROM url
            JOIN l_artist_url ON l_artist_url.entity0 = artist.id
                              AND l_artist_url.entity1 = url.id
          ) AS Links,
          array(
            SELECT INITCAP(genre.name)
            FROM genre
            JOIN tag ON genre.name = tag.name
            JOIN artist_tag ON artist_tag.tag = tag.id
            WHERE artist_tag.artist = artist.id
              AND artist_tag.count > 0
          ) AS Genres
        FROM artist
        LEFT JOIN artist_type ON artist.type = artist_type.id
        LEFT JOIN artist_meta ON artist.id = artist_meta.id
        WHERE artist.gid IN (
          SELECT DISTINCT artist.gid
          FROM artist
          JOIN artist_credit_name ON artist_credit_name.artist = artist.id
          JOIN track ON track.artist_credit = artist_credit_name.artist_credit
          JOIN medium ON track.medium = medium.id
          JOIN release ON medium.release = release.id
          WHERE release.release_group = release_group.id
            AND artist_credit_name.position = 0
          UNION
          SELECT artist.gid
          FROM artist
          JOIN artist_credit_name ON artist_credit_name.artist = artist.id
          WHERE artist_credit_name.artist_credit = release_group.artist_credit
            AND artist_credit_name.position = 0
        )
      ) artist_data
    ) AS Artists,
    json_build_object(
      'Count', COALESCE(release_group_meta.rating_count, 0),
      'Value', release_group_meta.rating::decimal / 10
    ) AS Rating,
    array(
      SELECT url.url
      FROM url
      JOIN l_release_group_url ON l_release_group_url.entity0 = release_group.id
                                AND l_release_group_url.entity1 = url.id
    ) AS Links,
    array(
      SELECT INITCAP(genre.name)
      FROM genre
      JOIN tag ON genre.name = tag.name
      JOIN release_group_tag ON release_group_tag.tag = tag.id
      WHERE release_group_tag.release_group = release_group.id
        AND release_group_tag.count > 0
    ) AS Genres,
    (
      SELECT json_agg(row_to_json(images_data))
      FROM (
        SELECT unnest(types) AS type,
               release.gid AS release_gid,
               index_listing.id AS image_id
        FROM cover_art_archive.index_listing
        JOIN release ON index_listing.release = release.id
        WHERE release.release_group = release_group.id
        ORDER BY index_listing.ordering ASC
      ) images_data
    ) AS Images,
    (
      SELECT COALESCE(json_agg(row_to_json(releases_data)), '[]'::json)
      FROM (
        SELECT
          release.gid AS Id,
          array(
            SELECT gid
            FROM release_gid_redirect
            WHERE release_gid_redirect.new_id = release.id
          ) AS OldIds,
          release.name AS Title,
          release.comment AS Disambiguation,
          release_status.name AS Status,
          (
            SELECT COALESCE(
                     MIN(make_date(date_year, date_month, date_day)),
                     MIN(make_date(COALESCE(date_year, 1), COALESCE(date_month, 1), COALESCE(date_day, 1)))
                   )
            FROM (
              SELECT date_year, date_month, date_day
              FROM release_country
              WHERE release_country.release = release.id
              UNION
              SELECT date_year, date_month, date_day
              FROM release_unknown_country
              WHERE release_unknown_country.release = release.id
            ) dates
          ) AS ReleaseDate,
          array(
            SELECT name
            FROM label
            JOIN release_label ON release_label.label = label.id
            WHERE release_label.release = release.id
            ORDER BY name ASC
          ) AS Label,
          array(
            SELECT name
            FROM area
            JOIN country_area ON country_area.area = area.id
            JOIN release_country ON release_country.country = country_area.area
            WHERE release_country.release = release.id
          ) AS Country,
          array(
            SELECT json_build_object(
              'Format', medium_format.name,
              'Name', medium.name,
              'Position', medium.position
            )
            FROM medium
            JOIN medium_format ON medium_format.id = medium.format
            WHERE medium.release = release.id
            ORDER BY medium.position
          ) AS Media,
          (SELECT SUM(medium.track_count) FROM medium WHERE medium.release = release.id) AS TrackCount,
          (
            SELECT COALESCE(json_agg(row_to_json(track_data)), '[]'::json)
            FROM (
              SELECT
                track.gid AS Id,
                array(
                  SELECT gid
                  FROM track_gid_redirect
                  WHERE track_gid_redirect.new_id = track.id
                ) AS OldIds,
                recording.gid AS RecordingId,
                array(
                  SELECT gid
                  FROM recording_gid_redirect
                  WHERE recording_gid_redirect.new_id = recording.id
                ) AS OldRecordingIds,
                artist.gid AS ArtistId,
                track.name AS TrackName,
                track.length AS DurationMs,
                medium.position AS MediumNumber,
                track.number AS TrackNumber,
                track.position AS TrackPosition
              FROM track
              JOIN medium ON track.medium = medium.id
              JOIN artist_credit_name ON artist_credit_name.artist_credit = track.artist_credit
              JOIN artist ON artist_credit_name.artist = artist.id
              JOIN recording ON track.recording = recording.id
              WHERE medium.release = release.id
                AND artist_credit_name.position = 0
                AND recording.video = FALSE
                AND track.is_data_track = FALSE
            ) track_data
          ) AS Tracks
        FROM release
        JOIN release_status ON release_status.id = release.status
        WHERE release.release_group = release_group.id
      ) releases_data
    ) AS Releases
  FROM release_group
  LEFT JOIN release_group_meta ON release_group_meta.id = release_group.id
  LEFT JOIN release_group_primary_type ON release_group.type = release_group_primary_type.id
  LEFT JOIN artist_credit_name ON artist_credit_name.artist_credit = release_group.artist_credit
  LEFT JOIN artist ON artist_credit_name.artist = artist.id
  LEFT JOIN artist_type ON artist.type = artist_type.id
  LEFT JOIN artist_meta ON artist.id = artist_meta.id
  JOIN metadada.releases_sync s ON release_group.gid = s.id
  WHERE s.sync IS FALSE
  LIMIT $1
) album_data;
