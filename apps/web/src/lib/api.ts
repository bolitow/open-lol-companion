import type { ApiErrorCode, BuildsResponse, Profile, ProfileMatches, StaticManifest, TierlistResponse } from "@olc/shared";
import { statsSearchParams, type StatsFilters } from "./filters";
import type { SiteErrorCode } from "./i18n";
import type { RiotId } from "./riotId";

/** Résultat d'un appel : jamais de corps brut d'erreur, seulement un code traduisible. */
export type ApiResult<T> = { ok: true; data: T } | { ok: false; code: SiteErrorCode };

export interface ApiClientOptions {
  /** Origine de `services/api`, par exemple `http://127.0.0.1:3030`. */
  baseUrl: string;
  /** Jeton de lecture émis côté serveur ; `null` désactive les routes dynamiques. */
  token: string | null;
  fetchImpl?: typeof fetch;
}

// `next.revalidate` est lu par le `fetch` de Next.js et ignoré ailleurs.
type FetchInit = RequestInit & { next?: { revalidate: number } };

const API_ERRORS: readonly ApiErrorCode[] = ["invalid_request", "unauthorized", "not_found", "unavailable", "rate_limited"];
/** Les documents statiques changent au plus à chaque version ; l'API les sert avec un cache d'une heure. */
const STATIC_REVALIDATE_S = 3600;
const MATCHES_PER_PAGE = 10;

/** Client de l'API interne (#19), destiné au seul code serveur du site. */
export function createApiClient({ baseUrl, token, fetchImpl = fetch }: ApiClientOptions) {
  const origin = baseUrl.replace(/\/+$/, "");

  async function request<T>(path: string, init: FetchInit): Promise<ApiResult<T>> {
    let response: Response;
    try {
      response = await fetchImpl(`${origin}${path}`, init);
    } catch {
      return { ok: false, code: "unavailable" };
    }
    let body: unknown = null;
    try {
      body = await response.json();
    } catch {
      body = null;
    }
    if (response.ok && body !== null) return { ok: true, data: body as T };
    const code = (body as { error?: { code?: unknown } } | null)?.error?.code;
    return { ok: false, code: API_ERRORS.find((known) => known === code) ?? "unavailable" };
  }

  /** Routes protégées : jeton en en-tête, réponses `no-store` comme l'API. */
  function dynamic<T>(path: string): Promise<ApiResult<T>> {
    if (token === null) return Promise.resolve({ ok: false, code: "not_configured" });
    return request<T>(path, { headers: { Authorization: `Bearer ${token}` }, cache: "no-store" });
  }

  /** Routes publiques : aucun jeton, cache revalidé. */
  function publicResource<T>(path: string): Promise<ApiResult<T>> {
    return request<T>(path, { next: { revalidate: STATIC_REVALIDATE_S } });
  }

  const riotIdPath = (platform: string, id: RiotId) =>
    `/v1/profiles/${encodeURIComponent(platform)}/${encodeURIComponent(id.gameName)}/${encodeURIComponent(id.tagLine)}`;

  return {
    manifest: () => publicResource<StaticManifest>("/v1/static/manifest"),
    staticDocument: (version: string, locale: "fr_FR" | "en_US", resource: string) =>
      publicResource<unknown>(`/v1/static/${encodeURIComponent(version)}/${locale}/${resource}`),
    tierlist: (filters: StatsFilters & { patch: string }) =>
      dynamic<TierlistResponse>(`/v1/tierlist?${statsSearchParams(filters).toString()}`),
    builds: (filters: StatsFilters & { patch: string }, championId: number) =>
      dynamic<BuildsResponse>(`/v1/builds/${championId}?${statsSearchParams(filters).toString()}`),
    profile: (platform: string, id: RiotId) => dynamic<Profile>(riotIdPath(platform, id)),
    matches: (platform: string, id: RiotId, start: number) =>
      dynamic<ProfileMatches>(`${riotIdPath(platform, id)}/matches?start=${start}&count=${MATCHES_PER_PAGE}`),
  };
}

export type ApiClient = ReturnType<typeof createApiClient>;
