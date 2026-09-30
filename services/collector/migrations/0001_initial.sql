-- Schéma initial du collecteur (#17) : exécutions, seeds, travaux reprenables,
-- parties et timelines. Les réponses Riot sont conservées en JSONB pour pouvoir
-- recalculer les données dérivées (#18) sans retélécharger.

CREATE TABLE collection_runs (
    id              BIGSERIAL PRIMARY KEY,
    status          TEXT NOT NULL
                    CHECK (status IN ('running', 'paused', 'completed', 'incomplete')),
    status_reason   TEXT,
    platform_id     TEXT NOT NULL,
    queue_id        INTEGER NOT NULL,
    -- Fenêtre figée au lancement, réutilisée telle quelle à la reprise.
    window_start    TIMESTAMPTZ NOT NULL,
    window_end      TIMESTAMPTZ NOT NULL CHECK (window_end > window_start),
    target_matches  INTEGER NOT NULL CHECK (target_matches > 0),
    params          JSONB NOT NULL,
    calls_made      BIGINT NOT NULL DEFAULT 0,
    started_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    finished_at     TIMESTAMPTZ,
    report          JSONB
);

-- Instantané du classement au moment de la collecte : ce rang n'est ni celui
-- du joueur au moment de ses parties, ni celui des autres participants.
CREATE TABLE seed_players (
    run_id          BIGINT NOT NULL REFERENCES collection_runs (id),
    puuid           TEXT NOT NULL,
    platform_id     TEXT NOT NULL,
    tier            TEXT NOT NULL,
    division        TEXT NOT NULL,
    league_points   INTEGER,
    source_page     INTEGER NOT NULL,
    seed_index      INTEGER NOT NULL,
    observed_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (run_id, puuid)
);

CREATE TABLE collection_jobs (
    id               BIGSERIAL PRIMARY KEY,
    run_id           BIGINT NOT NULL REFERENCES collection_runs (id),
    kind             TEXT NOT NULL
                     CHECK (kind IN ('seed_page', 'match_ids', 'match', 'timeline')),
    job_key          TEXT NOT NULL,
    payload          JSONB NOT NULL,
    state            TEXT NOT NULL DEFAULT 'pending'
                     CHECK (state IN ('pending', 'running', 'retry_wait', 'done', 'failed')),
    -- Ordre de traitement : répartit l'échantillon entre les rangs et les joueurs.
    sort_key         BIGINT NOT NULL DEFAULT 0,
    attempts         INTEGER NOT NULL DEFAULT 0,
    not_found_count  INTEGER NOT NULL DEFAULT 0,
    next_attempt_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- Résultat d'un travail terminé : stored, already_present, excluded:<raison>…
    outcome          TEXT,
    -- Message filtré : jamais de clé, d'URL ni de PUUID.
    last_error       TEXT,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (run_id, kind, job_key)
);

CREATE INDEX collection_jobs_claim_idx
    ON collection_jobs (run_id, state, next_attempt_at);

CREATE TABLE matches (
    match_id         TEXT PRIMARY KEY,
    platform_id      TEXT NOT NULL,
    queue_id         INTEGER NOT NULL,
    game_version     TEXT NOT NULL,
    patch            TEXT NOT NULL,
    game_start       TIMESTAMPTZ NOT NULL,
    game_duration_s  INTEGER NOT NULL,
    is_remake        BOOLEAN NOT NULL,
    data_version     TEXT,
    detail           JSONB NOT NULL,
    first_run_id     BIGINT NOT NULL REFERENCES collection_runs (id),
    fetched_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX matches_patch_idx ON matches (patch);
CREATE INDEX matches_queue_start_idx ON matches (queue_id, game_start);

-- Seuls les états définitifs sont ici ; « à réessayer » et « en échec » se lisent
-- dans collection_jobs.
CREATE TABLE match_timelines (
    match_id         TEXT PRIMARY KEY REFERENCES matches (match_id),
    status           TEXT NOT NULL CHECK (status IN ('available', 'unavailable')),
    timeline         JSONB,
    fetched_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK ((status = 'available') = (timeline IS NOT NULL))
);

-- Chaque fois qu'un historique mentionne une partie : permet de compter les doublons
-- et de retrouver la provenance.
CREATE TABLE run_discoveries (
    run_id          BIGINT NOT NULL REFERENCES collection_runs (id),
    match_id        TEXT NOT NULL,
    seed_puuid      TEXT NOT NULL,
    discovered_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (run_id, match_id, seed_puuid)
);

-- Échantillon retenu par une exécution, y compris les parties déjà en base.
CREATE TABLE run_matches (
    run_id          BIGINT NOT NULL REFERENCES collection_runs (id),
    match_id        TEXT NOT NULL REFERENCES matches (match_id),
    seed_puuid      TEXT NOT NULL,
    already_present BOOLEAN NOT NULL,
    linked_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (run_id, match_id)
);
