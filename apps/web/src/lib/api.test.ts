import { describe, expect, it } from "vitest";
import { createApiClient } from "./api";

type Call = { url: string; init: RequestInit | undefined };

function fakeFetch(status: number, body: unknown, calls: Call[]): typeof fetch {
  return (async (url: string | URL | Request, init?: RequestInit) => {
    calls.push({ url: String(url), init });
    return new Response(JSON.stringify(body), { status, headers: { "content-type": "application/json" } });
  }) as typeof fetch;
}

const filters = { patch: "16.19", platform: "EUW1", queue: 420, role: "MIDDLE", rank: "ALL" } as const;

describe("createApiClient", () => {
  it("authentifie les routes dynamiques par en-tête, jamais dans l'URL", async () => {
    const calls: Call[] = [];
    const client = createApiClient({ baseUrl: "http://api.test/", token: "jeton-de-test", fetchImpl: fakeFetch(200, { entries: [] }, calls) });
    const result = await client.tierlist(filters);
    expect(result.ok).toBe(true);
    expect(calls[0]?.url).toBe(
      "http://api.test/v1/tierlist?patch=16.19&platform=EUW1&queue=420&role=MIDDLE&rank=ALL&offset=0&limit=200",
    );
    expect(calls[0]?.url).not.toContain("jeton-de-test");
    expect(new Headers(calls[0]?.init?.headers).get("authorization")).toBe("Bearer jeton-de-test");
  });

  it("n'envoie pas le jeton aux routes statiques publiques", async () => {
    const calls: Call[] = [];
    const client = createApiClient({ baseUrl: "http://api.test", token: "jeton-de-test", fetchImpl: fakeFetch(200, {}, calls) });
    await client.staticDocument("16.19.1", "fr_FR", "champion.json");
    expect(calls[0]?.url).toBe("http://api.test/v1/static/16.19.1/fr_FR/champion.json");
    expect(new Headers(calls[0]?.init?.headers).has("authorization")).toBe(false);
  });

  it("encode séparément les segments du Riot ID", async () => {
    const calls: Call[] = [];
    const client = createApiClient({ baseUrl: "http://api.test", token: "t", fetchImpl: fakeFetch(200, {}, calls) });
    await client.profile("EUW1", { gameName: "Le Joueur/Élu", tagLine: "FR#1" });
    await client.matches("EUW1", { gameName: "Le Joueur/Élu", tagLine: "FR1" }, 20);
    expect(calls[0]?.url).toBe("http://api.test/v1/profiles/EUW1/Le%20Joueur%2F%C3%89lu/FR%231");
    expect(calls[1]?.url).toBe("http://api.test/v1/profiles/EUW1/Le%20Joueur%2F%C3%89lu/FR1/matches?start=20&count=10");
  });

  it("renvoie le code d'erreur de l'API sans corps brut", async () => {
    const calls: Call[] = [];
    const client = createApiClient({ baseUrl: "http://api.test", token: "t", fetchImpl: fakeFetch(503, { error: { code: "unavailable" } }, calls) });
    expect(await client.builds(filters, 103)).toEqual({ ok: false, code: "unavailable" });
    const notFound = createApiClient({ baseUrl: "http://api.test", token: "t", fetchImpl: fakeFetch(404, { error: { code: "not_found" } }, calls) });
    expect(await notFound.profile("EUW1", { gameName: "Inconnu", tagLine: "EUW" })).toEqual({ ok: false, code: "not_found" });
  });

  it("traite un corps d'erreur inattendu ou une panne réseau comme une indisponibilité", async () => {
    const odd = createApiClient({ baseUrl: "http://api.test", token: "t", fetchImpl: fakeFetch(500, "<html>", []) });
    expect(await odd.manifest()).toEqual({ ok: false, code: "unavailable" });
    const down = createApiClient({
      baseUrl: "http://api.test",
      token: "t",
      fetchImpl: (async () => {
        throw new TypeError("fetch failed");
      }) as typeof fetch,
    });
    expect(await down.manifest()).toEqual({ ok: false, code: "unavailable" });
  });

  it("signale une configuration absente sans appeler le réseau", async () => {
    const calls: Call[] = [];
    const client = createApiClient({ baseUrl: "http://api.test", token: null, fetchImpl: fakeFetch(200, {}, calls) });
    expect(await client.tierlist(filters)).toEqual({ ok: false, code: "not_configured" });
    expect(calls).toHaveLength(0);
  });
});
