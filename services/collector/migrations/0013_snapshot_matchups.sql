-- Matchups de lane (#123) : huitième section des instantanés v2, liste reprise de 0012.
-- La colonne `populations` (0009) indexe `champion_id` (le sujet) ; `opponent_champion_id`
-- n'en fait pas partie, aucun recalcul nécessaire.
ALTER TABLE champion_stats_snapshot_chunks
    DROP CONSTRAINT champion_stats_snapshot_chunks_section_check,
    ADD CONSTRAINT champion_stats_snapshot_chunks_section_check CHECK (section IN
        ('coverage', 'groups', 'bans', 'builds', 'skill_levels', 'item_events', 'performance',
         'matchups'));

-- La tête v2 reste limitée aux métadonnées, nouvelle section comprise.
ALTER TABLE champion_stats_snapshot
    DROP CONSTRAINT snapshot_metadata_only,
    ADD CONSTRAINT snapshot_metadata_only CHECK (storage_version = 1 OR NOT (report ?| ARRAY[
        'coverage', 'groups', 'bans', 'builds', 'skill_levels', 'item_events', 'performance',
        'matchups']));
