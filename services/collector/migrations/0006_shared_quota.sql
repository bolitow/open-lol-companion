-- Coordination API/collecteur : aucun identifiant joueur ni clé dans ces états.
CREATE TABLE riot_shared_quota (
    host TEXT PRIMARY KEY,
    state JSONB NOT NULL CHECK (jsonb_typeof(state) = 'object')
);
