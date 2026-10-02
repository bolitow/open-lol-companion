-- Les dimensions sont calculées une seule fois à l'écriture, pas à chaque lecture API.
-- Garder chaque combinaison entière évite de mélanger les dimensions de deux populations.
CREATE FUNCTION snapshot_chunk_populations(chunk_section TEXT, entries JSONB)
RETURNS JSONB LANGUAGE SQL IMMUTABLE STRICT PARALLEL SAFE AS $$
    SELECT jsonb_build_object(chunk_section, COALESCE(jsonb_agg(DISTINCT
        jsonb_strip_nulls(jsonb_build_object(
            'patch', item->'patch', 'platform_id', item->'platform_id',
            'queue_id', item->'queue_id', 'role', item->'role',
            'rank', item->'rank', 'champion_id', item->'champion_id'
        ))), '[]'::jsonb))
    FROM jsonb_array_elements(entries) AS item
$$;

-- PostgreSQL renseigne aussi les morceaux v2 déjà publiés. Aucun recalcul des
-- statistiques ni changement des JSON publics ; les prochains INSERT/UPDATE restent cohérents.
ALTER TABLE champion_stats_snapshot_chunks ADD COLUMN populations JSONB
    GENERATED ALWAYS AS (snapshot_chunk_populations(section, items)) STORED NOT NULL;

-- La section fait partie de la clé JSON : une lecture de couverture n'ouvre pas
-- tous les morceaux de builds ayant le même patch/serveur/file.
CREATE INDEX snapshot_chunk_populations_idx ON champion_stats_snapshot_chunks
    USING gin (populations jsonb_path_ops);
