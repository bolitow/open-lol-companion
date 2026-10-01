-- Campagnes bornées : la reprise conserve les runs, la fenêtre et l'échéance initiale.
CREATE TABLE collection_campaigns (
    id              BIGSERIAL PRIMARY KEY,
    created_at      TIMESTAMPTZ NOT NULL,
    deadline_at     TIMESTAMPTZ NOT NULL CHECK (deadline_at > created_at),
    status          TEXT NOT NULL CHECK (status IN ('ready', 'running', 'paused', 'finished')),
    status_reason   TEXT,
    params          JSONB NOT NULL,
    next_ordinal    INTEGER NOT NULL DEFAULT 0 CHECK (next_ordinal >= 0),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE TABLE campaign_runs (
    campaign_id     BIGINT NOT NULL REFERENCES collection_campaigns (id),
    run_id          BIGINT NOT NULL UNIQUE REFERENCES collection_runs (id),
    ordinal         INTEGER NOT NULL CHECK (ordinal >= 0),
    PRIMARY KEY (campaign_id, ordinal)
);
