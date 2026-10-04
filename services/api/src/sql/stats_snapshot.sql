-- Les clés constantes de la requête sélectionnent les morceaux par l'index GIN.
-- Un patch nul (série entre patchs) est retiré du filtre : tous les patchs sont lus.
-- Bans/couverture portent un périmètre ; les autres listes portent une population.
WITH filters AS (
    SELECT
        jsonb_build_array(jsonb_strip_nulls(jsonb_build_object(
            'patch', $1::jsonb->'patch', 'platform_id', $1->'platform', 'queue_id', $1->'queue'
        ))) AS scope,
        jsonb_build_array(jsonb_strip_nulls(jsonb_build_object(
            'patch', $1->'patch', 'platform_id', $1->'platform', 'queue_id', $1->'queue',
            'role', $1->'role', 'rank', $1->'rank', 'champion_id', $1->'champion'
        ))) AS population
)
SELECT s.source_snapshot_at::text, s.published_at::text, s.storage_version,
    CASE WHEN s.storage_version=1 THEN
        (s.report - ARRAY['groups','bans','builds','skill_levels','item_events','splits','coverage']) ||
        jsonb_build_object(
            'groups', jsonb_path_query_array(s.report->'groups',$2::jsonpath,$1),
            'bans', jsonb_path_query_array(s.report->'bans',$3::jsonpath,$1),
            'coverage', jsonb_path_query_array(s.report->'coverage',$3::jsonpath,$1),
            'builds', CASE WHEN $4 THEN jsonb_path_query_array(s.report->'builds',$2::jsonpath,$1) ELSE '[]'::jsonb END,
            'skill_levels', CASE WHEN $4 THEN jsonb_path_query_array(s.report->'skill_levels',$2::jsonpath,$1) ELSE '[]'::jsonb END,
            'item_events', CASE WHEN $4 THEN jsonb_path_query_array(s.report->'item_events',$2::jsonpath,$1) ELSE '[]'::jsonb END,
            -- Rapport antérieur à #119 : pas de clé, donc liste vide plutôt que null.
            'splits', CASE WHEN $4 THEN COALESCE(jsonb_path_query_array(s.report->'splits',$2::jsonpath,$1),'[]'::jsonb) ELSE '[]'::jsonb END
        )
    ELSE s.report || jsonb_build_object('groups','[]'::jsonb,'bans','[]'::jsonb,
        'coverage','[]'::jsonb,'builds','[]'::jsonb,'skill_levels','[]'::jsonb,'item_events','[]'::jsonb,
        'splits','[]'::jsonb)
    END AS report, c.section,
    CASE WHEN c.section IN ('coverage','bans') THEN jsonb_path_query_array(c.items,$3::jsonpath,$1)
        ELSE jsonb_path_query_array(c.items,$2::jsonpath,$1) END AS items
FROM champion_stats_snapshot s
CROSS JOIN filters f
LEFT JOIN champion_stats_snapshot_chunks c
    ON c.snapshot_id=s.id AND s.storage_version=2
    AND c.populations @> ANY (
        ARRAY[
            jsonb_build_object('coverage', f.scope),
            jsonb_build_object('bans', f.scope),
            jsonb_build_object('groups', f.population)
        ] || CASE WHEN $4 THEN ARRAY[
            jsonb_build_object('builds', f.population),
            jsonb_build_object('skill_levels', f.population),
            jsonb_build_object('item_events', f.population),
            jsonb_build_object('splits', f.population)
        ] ELSE ARRAY[]::jsonb[] END
    )
WHERE s.id=1
ORDER BY c.section,c.chunk_index
