-- Section `splits` (#119) : winrate par tranche de durée et par côté, publié en morceaux
-- comme les autres listes. Seule la contrainte de section change ; l'index des populations
-- et la colonne générée `populations` s'appliquent déjà à toute section.
-- Les six sections existantes sont conservées à l'identique.
ALTER TABLE champion_stats_snapshot_chunks
    DROP CONSTRAINT champion_stats_snapshot_chunks_section_check;
ALTER TABLE champion_stats_snapshot_chunks
    ADD CONSTRAINT champion_stats_snapshot_chunks_section_check CHECK (section IN
        ('coverage', 'groups', 'bans', 'builds', 'skill_levels', 'item_events', 'splits'));
