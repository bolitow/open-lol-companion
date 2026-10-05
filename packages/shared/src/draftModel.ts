/**
 * Modèle de draft, première version (#39, cahier §5.2 et §10.3).
 *
 * Calcul pur et déterministe sur les statistiques déjà publiées par l'API (#18/#19) :
 * aucun entraînement, aucune donnée cachée, aucun appel réseau. Le résultat décrit
 * l'échantillon publié ; il ne choisit rien à la place du joueur et ne prédit pas
 * l'issue d'une partie. Aucun poste n'est attribué aux adversaires : leur poste n'est
 * pas visible en sélection et la projection Rust le force à vide (revue #30).
 */
import type { Role } from "./api";
import type { DraftChampionStats } from "./draftStats";
import type { DraftPlayer } from "./draft";

/** Population unique d'où proviennent toutes les statistiques comparées. */
export interface DraftPopulation {
  patch: string;
  platform_id: string;
  queue_id: number;
  rank: string;
}

/** Champion allié visible ; `role` est le poste affiché par le client, sinon `null`. */
export interface DraftAllyPick {
  champion_id: number;
  role: Role | null;
  local: boolean;
}

export interface DraftModelInput {
  population: DraftPopulation;
  /** Seuil publié (`meta.min_games`) appliqué aux taux recombinés. */
  min_games: number;
  /** Poste pour lequel les candidats sont notés (poste visible du joueur local). */
  role: Role;
  /** Entrées publiées de la tierlist, tous rôles confondus ; les autres populations sont ignorées. */
  stats: DraftChampionStats[];
  allies: DraftAllyPick[];
  /** Champions adverses visibles, sans poste par construction. */
  enemies: number[];
  bans: number[];
}

/** Candidat noté ; `score` est la borne basse de Wilson à 95 % publiée, en pourcentage. */
export interface DraftCandidate {
  champion_id: number;
  role: Role;
  score: number | null;
  /** Rang descriptif parmi les candidats notés, `null` sous le seuil d'échantillon. */
  position: number | null;
  games: number;
  win_rate: number | null;
  pick_rate: number | null;
  /** Part des parties du champion jouées à ce poste, parmi les cinq postes connus ; `null` pour `UNKNOWN`. */
  role_share: number | null;
}

/** Résumé d'un camp ; l'estimation reste nulle si l'un des deux camps n'a aucun champion éligible. */
export interface DraftTeamEstimate {
  champions: number;
  eligible: number;
  mean_win_rate: number | null;
  estimated_win_rate: number | null;
}

export interface DraftModelResult {
  /** Identifiant technique de la méthode, à ne pas afficher tel quel. */
  method: string;
  population: DraftPopulation;
  min_games: number;
  role: Role;
  /** Tous les candidats disponibles, notés d'abord ; aucune recommandation unique. */
  candidates: DraftCandidate[];
  teams: { ally: DraftTeamEstimate; enemy: DraftTeamEstimate };
  matchups_available: boolean;
  synergies_available: boolean;
}

export const DRAFT_MODEL_METHOD =
  "v1 descriptive: candidate score = published Wilson95 lower bound in role; " +
  "team estimate = team mean published win rate / sum of both team means * 100; " +
  "allies by visible role, enemies and allies without role across all roles; " +
  "matchups and synergies not included";

const KNOWN_ROLES: readonly Role[] = ["TOP", "JUNGLE", "MIDDLE", "BOTTOM", "UTILITY"];

/** Convertit le poste visible de la projection de sélection vers le rôle des statistiques. */
export function roleFromDraftPosition(position: DraftPlayer["position"]): Role | null {
  return position === null ? null : (position.toUpperCase() as Role);
}

/** Note les candidats d'un poste et résume les deux camps à partir des statistiques publiées. */
export function scoreDraft(input: DraftModelInput): DraftModelResult {
  const { population, role } = input;
  const stats = input.stats.filter(
    (s) =>
      s.patch === population.patch &&
      s.platform_id === population.platform_id &&
      s.queue_id === population.queue_id &&
      s.rank === population.rank,
  );
  const byChampion = new Map<number, DraftChampionStats[]>();
  for (const s of stats) {
    byChampion.set(s.champion_id, [...(byChampion.get(s.champion_id) ?? []), s]);
  }

  const unavailable = new Set<number>([
    ...input.bans,
    ...input.enemies,
    ...input.allies.filter((a) => !a.local).map((a) => a.champion_id),
  ]);
  const candidates: DraftCandidate[] = stats
    .filter((s) => s.role === role && !unavailable.has(s.champion_id))
    .map((s) => {
      const known = (byChampion.get(s.champion_id) ?? []).filter((e) => KNOWN_ROLES.includes(e.role));
      const knownGames = known.reduce((sum, e) => sum + e.games, 0);
      return {
        champion_id: s.champion_id,
        role: s.role,
        score: s.win_rate_lower_bound,
        position: null,
        games: s.games,
        win_rate: s.win_rate,
        pick_rate: s.pick_rate,
        // UNKNOWN n'entre pas dans le dénominateur : sa part n'aurait pas de sens (> 100 possible).
        role_share:
          KNOWN_ROLES.includes(role) && knownGames > 0 ? (100 * s.games) / knownGames : null,
      };
    })
    .sort(
      (a, b) =>
        (b.score ?? -1) - (a.score ?? -1) || b.games - a.games || a.champion_id - b.champion_id,
    );
  let position = 0;
  for (const c of candidates) {
    if (c.score !== null) c.position = ++position;
  }

  // Taux tous rôles confondus : les postes partitionnent les participations d'un champion
  // dans une même population, leur somme est donc légitime (ce ne sont pas des rangs).
  const anyRoleRate = (championId: number): number | null => {
    const entries = byChampion.get(championId) ?? [];
    const games = entries.reduce((sum, e) => sum + e.games, 0);
    const wins = entries.reduce((sum, e) => sum + e.wins, 0);
    return games > 0 && games >= input.min_games ? (100 * wins) / games : null;
  };
  const allyRates = input.allies.map((a) =>
    a.role === null
      ? anyRoleRate(a.champion_id)
      : (byChampion.get(a.champion_id)?.find((e) => e.role === a.role)?.win_rate ?? null),
  );
  const enemyRates = input.enemies.map(anyRoleRate);
  const ally = summarize(allyRates);
  const enemy = summarize(enemyRates);
  if (ally.mean_win_rate !== null && enemy.mean_win_rate !== null) {
    const total = ally.mean_win_rate + enemy.mean_win_rate;
    if (total > 0) {
      ally.estimated_win_rate = (100 * ally.mean_win_rate) / total;
      enemy.estimated_win_rate = (100 * enemy.mean_win_rate) / total;
    }
  }

  return {
    method: DRAFT_MODEL_METHOD,
    population,
    min_games: input.min_games,
    role,
    candidates,
    teams: { ally, enemy },
    matchups_available: false,
    synergies_available: false,
  };
}

function summarize(rates: (number | null)[]): DraftTeamEstimate {
  const eligible = rates.filter((r): r is number => r !== null);
  return {
    champions: rates.length,
    eligible: eligible.length,
    mean_win_rate:
      eligible.length > 0 ? eligible.reduce((sum, r) => sum + r, 0) / eligible.length : null,
    estimated_win_rate: null,
  };
}
