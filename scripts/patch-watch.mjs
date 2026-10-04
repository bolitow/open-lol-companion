// Détection d'un nouveau patch LoL pour le workflow planifié `patch-watch.yml` (#121).
//
// Lit uniquement la liste publique des versions Data Dragon : aucun appel Riot
// authentifié, aucune clé. La « dernière version contrôlée » est mémorisée par une issue
// GitHub titrée d'après le patch (la CI ne dispose d'aucun secret pour écrire une
// variable de dépôt) ; elle reste ainsi lisible par l'équipe.
import { execFileSync } from "node:child_process";
import { appendFileSync } from "node:fs";
import { pathToFileURL } from "node:url";

export const VERSIONS_URL = "https://ddragon.leagueoflegends.com/api/versions.json";

const VERSION_PATTERN = /^\d+\.\d+\.\d+$/;

/** Version la plus récente (Data Dragon trie du plus récent au plus ancien). */
export function latestVersion(versions) {
  const first = Array.isArray(versions) ? versions[0] : undefined;
  if (typeof first !== "string" || !VERSION_PATTERN.test(first)) {
    throw new Error("versions.json inattendu : première entrée absente ou illisible");
  }
  return first;
}

/** Patch « majeur.mineur » : un correctif (16.19.2) reste dans le patch 16.19. */
export function patchOf(version) {
  return version.split(".").slice(0, 2).join(".");
}

export function patchIssueTitle(patch) {
  return `Patch LoL ${patch} : contrôle automatique`;
}

/** Requête de recherche GitHub ; le résultat est refiltré sur le titre exact. */
export function issueSearchQuery(patch) {
  return `"Patch LoL ${patch}" in:title`;
}

/**
 * Décide s'il faut lancer les tests : oui pour un patch sans issue, ou si `force`.
 * `issueTitles` vient d'une recherche large ; seul un titre identique compte.
 */
export function decide({ versions, issueTitles, force }) {
  const version = latestVersion(versions);
  const patch = patchOf(version);
  const title = patchIssueTitle(patch);
  const known = issueTitles.includes(title);
  return { version, patch, title, shouldRun: force || !known };
}

async function main() {
  const force = process.env.FORCE === "true";
  const response = await fetch(VERSIONS_URL);
  if (!response.ok) {
    throw new Error(`Data Dragon a répondu ${response.status}`);
  }
  const versions = await response.json();
  const patch = patchOf(latestVersion(versions));
  const raw = execFileSync(
    "gh",
    [
      "issue", "list", "--state", "all", "--limit", "50",
      "--search", issueSearchQuery(patch),
      "--json", "title", "--jq", ".[].title",
    ],
    { encoding: "utf8" },
  );
  const issueTitles = raw.split("\n").filter(Boolean);
  const result = decide({ versions, issueTitles, force });
  console.log(JSON.stringify(result));
  if (process.env.GITHUB_OUTPUT) {
    const lines = Object.entries({
      version: result.version,
      patch: result.patch,
      title: result.title,
      should_run: String(result.shouldRun),
    }).map(([key, value]) => `${key}=${value}\n`);
    appendFileSync(process.env.GITHUB_OUTPUT, lines.join(""));
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main().catch((error) => {
    console.error(error.message);
    process.exit(1);
  });
}
