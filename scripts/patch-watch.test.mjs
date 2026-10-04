// Tests de la détection de patch LoL (#121). Exécutés par `node --test scripts`.
import assert from "node:assert/strict";
import { test } from "node:test";

import {
  decide,
  issueSearchQuery,
  latestVersion,
  patchIssueTitle,
  patchOf,
} from "./patch-watch.mjs";

// Extrait figé de versions.json : la plus récente d'abord, comme Data Dragon.
const VERSIONS = ["16.19.1", "16.18.1", "16.17.1", "lolpatch_3.7"];

test("latestVersion retient la première entrée de versions.json", () => {
  assert.equal(latestVersion(VERSIONS), "16.19.1");
});

test("latestVersion refuse une réponse qui n'est pas une liste de versions", () => {
  assert.throws(() => latestVersion([]), /versions\.json/);
  assert.throws(() => latestVersion({ version: "16.19.1" }), /versions\.json/);
  assert.throws(() => latestVersion(["n'importe quoi"]), /versions\.json/);
  assert.throws(() => latestVersion(["16.19.1; rm -rf /"]), /versions\.json/);
  assert.throws(() => latestVersion([16.19]), /versions\.json/);
});

test("patchOf ramène une version de correctif au patch majeur.mineur", () => {
  assert.equal(patchOf("16.19.1"), "16.19");
  assert.equal(patchOf("16.19.2"), "16.19");
  assert.equal(patchOf("16.9.1"), "16.9");
});

test("le titre de l'issue et sa requête de recherche portent le patch", () => {
  assert.equal(patchIssueTitle("16.19"), "Patch LoL 16.19 : contrôle automatique");
  assert.match(issueSearchQuery("16.19"), /"Patch LoL 16\.19"/);
  assert.match(issueSearchQuery("16.19"), /in:title/);
});

test("decide lance les tests pour un patch jamais contrôlé", () => {
  const result = decide({ versions: VERSIONS, issueTitles: [], force: false });
  assert.deepEqual(result, {
    version: "16.19.1",
    patch: "16.19",
    title: "Patch LoL 16.19 : contrôle automatique",
    shouldRun: true,
  });
});

test("decide ne relance pas un patch déjà tracé par une issue", () => {
  const titles = ["Autre sujet", patchIssueTitle("16.19")];
  assert.equal(decide({ versions: VERSIONS, issueTitles: titles, force: false }).shouldRun, false);
});

test("decide ne confond pas deux patchs aux numéros proches", () => {
  const titles = [patchIssueTitle("16.1"), patchIssueTitle("16.190")];
  assert.equal(decide({ versions: VERSIONS, issueTitles: titles, force: false }).shouldRun, true);
});

test("un correctif du même patch ne relance pas les tests", () => {
  const titles = [patchIssueTitle("16.19")];
  const hotfix = ["16.19.2", "16.19.1", "16.18.1"];
  assert.equal(decide({ versions: hotfix, issueTitles: titles, force: false }).shouldRun, false);
});

test("force relance les tests même pour un patch déjà tracé", () => {
  const titles = [patchIssueTitle("16.19")];
  assert.equal(decide({ versions: VERSIONS, issueTitles: titles, force: true }).shouldRun, true);
});
