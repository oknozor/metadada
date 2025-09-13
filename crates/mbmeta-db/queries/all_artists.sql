SELECT
  json_agg(artist_data) AS "items: Json<Vec<Artist>>"
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
        ) AS Genres,
        (
            SELECT json_agg(album_data)
            FROM (
                SELECT
                    release_group.gid AS Id,
                    array(
                        SELECT gid
                        FROM release_group_gid_redirect
                        WHERE release_group_gid_redirect.new_id = release_group.id
                    ) AS OldIds,
                    release_group.name AS Title,
                    COALESCE(release_group_primary_type.name, 'Other') AS Type,
                    array(
                        SELECT name
                        FROM release_group_secondary_type rgst
                        JOIN release_group_secondary_type_join rgstj
                          ON rgstj.secondary_type = rgst.id
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
                        SELECT COALESCE(json_agg(DISTINCT release_status.name), '[]'::json)
                        FROM release
                        JOIN release_status ON release_status.id = release.status
                        WHERE release.release_group = release_group.id
                    ) AS ReleaseStatuses,
                    json_build_object(
                        'Count', COALESCE(release_group_meta.rating_count, 0),
                        'Value', release_group_meta.rating::decimal / 10
                    ) AS Rating
                FROM release_group
                LEFT JOIN release_group_meta
                  ON release_group_meta.id = release_group.id
                LEFT JOIN release_group_primary_type
                  ON release_group.type = release_group_primary_type.id
                LEFT JOIN artist_credit_name
                  ON artist_credit_name.artist_credit = release_group.artist_credit
                WHERE artist_credit_name.artist = artist.id
                  AND artist_credit_name.position = 0
                ORDER BY release_group.gid
            ) album_data
        ) AS Albums
    FROM artist
    LEFT JOIN artist_type ON artist.type = artist_type.id
    LEFT JOIN artist_meta ON artist.id = artist_meta.id
    WHERE artist.gid > $1
    ORDER BY artist.gid
    LIMIT $2
) artist_data;
