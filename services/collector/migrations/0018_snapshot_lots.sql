-- Recalcul par lots (#89) : un lot est un périmètre patch/plateforme/file. Son empreinte
-- décide s'il est recalculé ; ses compteurs additifs reconstituent l'en-tête sans le relire.
CREATE TABLE champion_stats_snapshot_lots (
    snapshot_id SMALLINT NOT NULL REFERENCES champion_stats_snapshot(id) ON DELETE CASCADE,
    patch TEXT NOT NULL,
    platform_id TEXT NOT NULL,
    queue_id INTEGER NOT NULL,
    fingerprint TEXT NOT NULL,
    counts JSONB NOT NULL CHECK (jsonb_typeof(counts) = 'object'),
    -- Morceaux écrits pour ce lot : un écart avec la table des morceaux le fait recalculer.
    chunks INTEGER NOT NULL CHECK (chunks >= 0),
    PRIMARY KEY (snapshot_id, patch, platform_id, queue_id)
);

-- Lot d'origine d'un morceau ; NULL pour un recalcul complet, qui ne découpe pas par lot.
-- Supprimer un lot supprime ses morceaux. Les lecteurs (API) n'en dépendent pas.
ALTER TABLE champion_stats_snapshot_chunks
    ADD COLUMN lot_patch TEXT,
    ADD COLUMN lot_platform_id TEXT,
    ADD COLUMN lot_queue_id INTEGER,
    ADD CONSTRAINT snapshot_chunk_lot_complete CHECK (
        (lot_patch IS NULL) = (lot_platform_id IS NULL)
        AND (lot_patch IS NULL) = (lot_queue_id IS NULL)),
    ADD CONSTRAINT snapshot_chunk_lot_fk
        FOREIGN KEY (snapshot_id, lot_patch, lot_platform_id, lot_queue_id)
        REFERENCES champion_stats_snapshot_lots (snapshot_id, patch, platform_id, queue_id)
        ON DELETE CASCADE;
CREATE INDEX snapshot_chunk_lot_idx ON champion_stats_snapshot_chunks
    (snapshot_id, lot_patch, lot_platform_id, lot_queue_id);

-- Lecture d'un lot par pages ordonnées, sans parcourir les autres périmètres.
CREATE INDEX matches_lot_idx ON matches (patch, platform_id, queue_id, match_id);
-- Empreinte des rangs d'un lot classé : observations de sa file dans une fenêtre de dates.
CREATE INDEX participant_rank_window_idx
    ON participant_rank_observations (platform_id, queue_id, observed_at);
