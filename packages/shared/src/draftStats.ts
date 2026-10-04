import type {ChampionStats} from './api';
import type {BuildReport} from './builds';
/** Population lue par Rust ; ni champion choisi ni identité de joueur ne sont transmis. */
export interface DraftStatsRequest {patch:string;platform:string;queue:number;rank:string}
export type DraftChampionStats=Pick<ChampionStats,'patch'|'platform_id'|'queue_id'|'rank'|'role'|'champion_id'|'games'|'wins'|'win_rate'|'pick_rate'|'win_rate_lower_bound'>;
/** Six rôles complets issus d'une même publication, contrôlés avant exposition à React. */
export interface DraftStatsReport {meta:BuildReport['meta'];entries:DraftChampionStats[]}
