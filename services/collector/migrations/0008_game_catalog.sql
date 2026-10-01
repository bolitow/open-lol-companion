-- Sources immuables et publications par entité, indépendantes du cache brut #18.
CREATE TABLE game_catalog_sources (
    id TEXT PRIMARY KEY,
    -- JSON conserve la représentation numérique ; JSONB perd notamment le signe de -0.0.
    document JSON NOT NULL CHECK (json_typeof(document) = 'object')
);
CREATE TABLE game_catalog_publications (
    id TEXT PRIMARY KEY,
    version TEXT NOT NULL,
    normalizer_version INTEGER NOT NULL,
    published_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    manifest JSONB NOT NULL CHECK (jsonb_typeof(manifest) = 'object')
);
CREATE INDEX game_catalog_publications_version ON game_catalog_publications(version, published_at DESC);
CREATE TABLE game_catalog_source_refs (
    publication_id TEXT NOT NULL REFERENCES game_catalog_publications(id),
    source_id TEXT NOT NULL REFERENCES game_catalog_sources(id),
    PRIMARY KEY (publication_id, source_id)
);
CREATE TABLE game_catalog_entries (
    publication_id TEXT NOT NULL REFERENCES game_catalog_publications(id),
    kind TEXT NOT NULL,
    id TEXT NOT NULL,
    namespace TEXT NOT NULL,
    locale TEXT NOT NULL,
    name TEXT NOT NULL,
    data JSONB NOT NULL CHECK (jsonb_typeof(data) = 'object'),
    PRIMARY KEY (publication_id, kind, id, namespace, locale)
);
CREATE INDEX game_catalog_entries_page ON game_catalog_entries(publication_id, kind, locale, namespace, id);
CREATE TABLE game_catalog_current (
    version TEXT PRIMARY KEY,
    publication_id TEXT NOT NULL REFERENCES game_catalog_publications(id)
);
