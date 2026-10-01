-- Collecte #18 : observations de classement datées, distinctes des seeds.
ALTER TABLE collection_jobs DROP CONSTRAINT collection_jobs_kind_check;
ALTER TABLE collection_jobs ADD CONSTRAINT collection_jobs_kind_check
    CHECK (kind IN ('seed_page', 'match_ids', 'match', 'timeline', 'participant_rank'));

CREATE TABLE participant_rank_observations (
    id              BIGSERIAL PRIMARY KEY,
    platform_id     TEXT NOT NULL,
    puuid           TEXT NOT NULL CHECK (length(puuid) > 0),
    queue_id        INTEGER NOT NULL CHECK (queue_id IN (420, 440)),
    tier            TEXT,
    division        TEXT,
    league_points   INTEGER,
    observed_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    status          TEXT NOT NULL CHECK (status IN ('ranked', 'unranked')),
    CHECK ((status = 'ranked' AND tier IS NOT NULL AND division IS NOT NULL AND league_points IS NOT NULL)
        OR (status = 'unranked' AND tier IS NULL AND division IS NULL AND league_points IS NULL))
);
CREATE INDEX participant_rank_latest_idx
    ON participant_rank_observations (platform_id, puuid, queue_id, observed_at DESC);
