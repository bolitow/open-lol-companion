-- Section `splits` (#119) : winrate par tranche de durée et par côté, publié en morceaux
-- comme les autres listes. Seule la contrainte de section change ; l'index des populations
-- et la colonne générée `populations` s'appliquent déjà à toute section.
-- La liste reprend toutes les sections des migrations précédentes (0012 `performance`,
-- 0013 `matchups`) : jouée après elles, cette migration ne doit en retirer aucune.
ALTER TABLE champion_stats_snapshot_chunks
    DROP CONSTRAINT champion_stats_snapshot_chunks_section_check;
ALTER TABLE champion_stats_snapshot_chunks
    ADD CONSTRAINT champion_stats_snapshot_chunks_section_check CHECK (section IN
        ('coverage', 'groups', 'bans', 'builds', 'skill_levels', 'item_events', 'performance',
         'matchups', 'splits'));
-- La tête v2 reste limitée aux métadonnées, nouvelle section comprise.
ALTER TABLE champion_stats_snapshot
    DROP CONSTRAINT snapshot_metadata_only,
    ADD CONSTRAINT snapshot_metadata_only CHECK (storage_version = 1 OR NOT (report ?| ARRAY[
        'coverage', 'groups', 'bans', 'builds', 'skill_levels', 'item_events', 'performance',
        'matchups', 'splits']));
