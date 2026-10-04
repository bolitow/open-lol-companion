-- Rétention des données personnelles (#99) : date de retrait des identifiants de
-- joueurs du JSONB, et index des purges par âge et des recherches par PUUID.
ALTER TABLE matches ADD COLUMN identifiers_redacted_at TIMESTAMPTZ;
ALTER TABLE match_timelines ADD COLUMN identifiers_redacted_at TIMESTAMPTZ;

CREATE INDEX matches_fetched_idx ON matches (fetched_at);
CREATE INDEX match_timelines_fetched_idx ON match_timelines (fetched_at);
CREATE INDEX participant_rank_observed_idx ON participant_rank_observations (observed_at);
CREATE INDEX participant_rank_puuid_idx ON participant_rank_observations (puuid);
CREATE INDEX seed_players_puuid_idx ON seed_players (puuid);
CREATE INDEX run_discoveries_seed_idx ON run_discoveries (seed_puuid);
CREATE INDEX run_matches_seed_idx ON run_matches (seed_puuid);
-- Export et effacement sur demande : retrouver les parties d'un PUUID sans tout lire.
CREATE INDEX matches_participants_idx
    ON matches USING gin ((detail -> 'metadata' -> 'participants'));
CREATE INDEX match_timelines_participants_idx
    ON match_timelines USING gin ((timeline -> 'metadata' -> 'participants'));
