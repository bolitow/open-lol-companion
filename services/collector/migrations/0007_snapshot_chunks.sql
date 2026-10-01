-- La représentation JSONB d'un rapport complet peut dépasser 256 Mio.
-- Les anciennes publications restent lisibles jusqu'au prochain recalcul réussi.
ALTER TABLE champion_stats_snapshot ADD COLUMN storage_version SMALLINT NOT NULL DEFAULT 1
    CHECK (storage_version IN (1, 2));
-- Un ancien binaire ne doit pas remplacer silencieusement la tête d'un stockage v2.
ALTER TABLE champion_stats_snapshot ADD CONSTRAINT snapshot_metadata_only
    CHECK (storage_version = 1 OR NOT (report ?| ARRAY[
        'coverage', 'groups', 'bans', 'builds', 'skill_levels', 'item_events']));

CREATE TABLE champion_stats_snapshot_chunks (
    snapshot_id SMALLINT NOT NULL REFERENCES champion_stats_snapshot(id) ON DELETE CASCADE,
    section TEXT NOT NULL CHECK (section IN
        ('coverage', 'groups', 'bans', 'builds', 'skill_levels', 'item_events')),
    chunk_index INTEGER NOT NULL CHECK (chunk_index >= 0),
    items JSONB NOT NULL CHECK (jsonb_typeof(items) = 'array'),
    PRIMARY KEY (snapshot_id, section, chunk_index)
);
