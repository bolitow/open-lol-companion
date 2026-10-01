-- Premier lot #18 : publication atomique du dernier rapport complet.
-- Une seule ligne remplaçable ; aucune duplication des données personnelles brutes.
CREATE TABLE champion_stats_snapshot (
    id                  SMALLINT PRIMARY KEY CHECK (id = 1),
    source_snapshot_at  TIMESTAMPTZ NOT NULL,
    published_at        TIMESTAMPTZ NOT NULL,
    report              JSONB NOT NULL CHECK (jsonb_typeof(report) = 'object')
);
