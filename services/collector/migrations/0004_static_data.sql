-- Données publiques seulement : chaque bundle bilingue est complet avant publication.
CREATE TABLE static_data_releases (
    version TEXT PRIMARY KEY,
    patch TEXT NOT NULL,
    completed_at TIMESTAMPTZ NOT NULL,
    bundle JSONB NOT NULL CHECK (jsonb_typeof(bundle) = 'object')
);

-- Les catalogues queues/maps/modes/types ne sont pas versionnés par Riot.
-- Ils restent liés à la date de synchronisation, sans prétendre décrire un ancien patch.
CREATE TABLE static_data_manifest (
    id SMALLINT PRIMARY KEY CHECK (id = 1),
    live_version TEXT NOT NULL,
    checked_at TIMESTAMPTZ NOT NULL,
    versions JSONB NOT NULL CHECK (jsonb_typeof(versions) = 'array'),
    catalogs JSONB NOT NULL CHECK (jsonb_typeof(catalogs) = 'object')
);
