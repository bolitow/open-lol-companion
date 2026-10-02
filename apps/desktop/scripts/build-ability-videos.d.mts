export type VideoSource = {src: string; type: 'video/mp4' | 'video/webm'};
export type AbilityVideoEntry = {page: string; poster: string | null; width: number; height: number; sources: VideoSource[]};
export type ParsedAbilityVideo = AbilityVideoEntry & {id: string};
export type AbilityVideoManifest = {schemaVersion: 1; checkedAt: string; abilities: Record<string, AbilityVideoEntry>};
export function validateMediaUrl(value: string, type: string): {championId: number; slot: string};
export function parseRoster(html: string): string[];
export function parseOfficialPage(html: string, page: string): ParsedAbilityVideo[];
export function buildManifest(entries: ParsedAbilityVideo[], expectedChampionIds: number[], check: (url: string, type: string) => Promise<boolean>, checkedAt?: string): Promise<AbilityVideoManifest>;
export function generate(): Promise<void>;
