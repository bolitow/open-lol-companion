-- Cache négatif des parties exclues (#90) : une partie téléchargée puis écartée du
-- périmètre d'une exécution n'est pas retéléchargée par la suivante. On conserve les
-- faits indexés, jamais le détail ni d'identifiant de joueur : le verdict dépend du
-- périmètre (file, patch, fenêtre) et se recalcule donc à chaque exécution.
CREATE TABLE excluded_matches (
    match_id         TEXT PRIMARY KEY,
    platform_id      TEXT NOT NULL,
    queue_id         INTEGER NOT NULL,
    game_version     TEXT NOT NULL,
    patch            TEXT NOT NULL,
    game_start       TIMESTAMPTZ NOT NULL,
    game_duration_s  INTEGER NOT NULL,
    is_remake        BOOLEAN NOT NULL,
    data_version     TEXT,
    excluded_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
-- La purge supprime les lignes plus anciennes que la rétention des parties brutes (#99).
CREATE INDEX excluded_matches_excluded_at_idx ON excluded_matches (excluded_at);
