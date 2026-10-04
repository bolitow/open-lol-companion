# API interne REST + WebSocket — #19

Service Rust/Axum consommé par l'app (via son cœur Rust) et le site. PostgreSQL
fournit les agrégats et statiques #18 ; les profils utilisent les API officielles
Riot. Aucune donnée du client LCU ne transite par ce service.

## Démarrage

Rust stable et PostgreSQL 17. Axum requiert Rust ≥ 1.80 ; utiliser la toolchain
stable pour le workspace. Compléter le `.env` existant avec `.env.example`.
Commandes depuis la racine ; migrations du collecteur appliquées au démarrage.

| Variable | Usage |
| --- | --- |
| `DATABASE_URL` | Même base que le collecteur |
| `OLC_API_JWT_SECRET` | Secret aléatoire d'au moins 32 octets, exclusivement serveur |
| `OLC_API_JWT_ISSUER` | Émetteur attendu, défaut `open-lol-companion` |
| `OLC_API_JWT_AUDIENCE` | Audience attendue, défaut `olc-api` |
| `OLC_API_BIND` | Écoute, défaut `127.0.0.1:3030` |
| `OLC_API_ALLOWED_ORIGINS` | Origines web exactes, séparées par virgule ; aucune par défaut |
| `RIOT_API_KEY` | Facultative : profils indisponibles sans clé, agrégats/statiques disponibles |
| `OLC_API_PRIVACY_OPERATORS` | Sujets de jeton autorisés à l'export/effacement RGPD, séparés par virgule ; aucun par défaut |
| `OLC_TEST_DATABASE_URL` | Base d'administration des tests PostgreSQL jetables |

```sh
cargo run -p olc-api -- serve
```

Dans un autre terminal, émettre un jeton de développement **sur le serveur** :

```sh
export OLC_API_TOKEN="$(cargo run -q -p olc-api -- token --subject development --ttl 3600)"
curl -H "Authorization: Bearer $OLC_API_TOKEN" \
  'http://127.0.0.1:3030/v1/tierlist?patch=16.19&platform=EUW1&queue=420&role=MIDDLE'
```

PowerShell : `$env:OLC_API_TOKEN = cargo run -q -p olc-api -- token --subject development`.
Ne pas committer ni journaliser le jeton. Aucun endpoint public ne signe de jeton.
Un jeton autorise la lecture de l'API ; son sujet ne prouve pas la propriété d'un
compte Riot. Connexion utilisateur/RSO : intégration distincte. En production,
terminer HTTPS/WSS par un proxy, protéger l'émission et limiter le débit à l'entrée.
Une rotation du secret invalide les jetons.

## REST v1

Routes dynamiques : `Authorization: Bearer …`, signature HS256, expiration,
activation, émetteur, audience et sujet validés. Réponses dynamiques/erreurs
`no-store`. Aucun jeton en paramètre d'URL.

| Route GET | Données |
| --- | --- |
| `/health` | Connectivité PostgreSQL, publique |
| `/v1/tierlist` | Champions et bans de la page |
| `/v1/builds/{champion_id}` | Variantes, compétences et achats |
| `/v1/profiles/{platform}/{game_name}/{tag_line}` | Identité actuelle, icône, niveau, Solo/Flex horodatés, avec victoires/défaites et drapeaux league-v4 |
| `/v1/profiles/{platform}/{game_name}/{tag_line}/matches` | Historique du joueur recherché |
| `/v1/static/manifest` | Versions et catalogues publics |
| `/v1/static/{version}/{locale}/{resource}` | Document public, exemple `16.19.1/fr_FR/item.json` |
| `/v1/catalog/{version}/manifest` | Sources, couverture et inventaire de la publication normalisée |
| `/v1/catalog/{version}/{locale}/{kind}` | Fiches paginées, recherche et filtres de statistiques/prix/disponibilité |
| `/v1/catalog/{version}/{locale}/{kind}/{id}` | Fiche, valeurs sourcées et paramètres/limites des effets |
| `/v1/catalog-diff?from=…&to=…&locale=…&kind=…` | Diff paginé entre deux empreintes de publication |

### Données personnelles (#99)

| Route POST | Effet |
| --- | --- |
| `/v1/privacy/export` | Données détenues pour un PUUID : joueurs de départ, rangs observés, découvertes, parties avec **sa seule** fiche de participant, timelines, travaux de collecte |
| `/v1/privacy/erase` | Efface ce PUUID partout, en une transaction : lignes supprimées, identifiants retirés du JSONB des parties (les autres joueurs restent) |

Corps : `{"puuid":"…"}` ; le PUUID n'apparaît jamais dans l'URL. Réservées aux
sujets de `OLC_API_PRIVACY_OPERATORS` (sinon `forbidden`, avant toute requête SQL) :
le sujet d'un jeton ne prouve pas la propriété d'un compte Riot, l'opérateur vérifie
l'identité du demandeur hors de l'API puis obtient son PUUID par la route profil.
Effacement idempotent ; `jobs_in_flight` > 0 signale un travail de collecte en cours,
à relancer après la fin de la collecte. Pas de CORS : outil d'opérateur, pas du site.
Rétention planifiée : `olc-collector purge` ([README du collecteur](../collector/README.md)).

Les routes `catalog` sont publiques comme les statiques (ETag/304 et cache
revalidable). Contrats Rust/TypeScript, paramètres précis, langues FR/EN et
catalogues globaux `und/global` : [référentiel #61](../../docs/catalogue-jeu.md).

Encoder séparément les segments du Riot ID. Plateformes Riot : `EUW1`, `KR`…
Les origines configurées s'appliquent à CORS et WebSocket ; clients natifs sans
Origin acceptés. Les contrats sont dans [`api.ts`](../../packages/shared/src/api.ts).

### Statistiques

Requis : `patch` technique (`16.19`), `platform`, `queue`, `role`
(`TOP/JUNGLE/MIDDLE/BOTTOM/UTILITY/UNKNOWN`). `rank=ALL` par défaut, ou rang Riot,
`UNKNOWN`, `UNRANKED`, `UNRANKED_MODE`. Ne jamais additionner ces populations.
`offset=0`, `limit=50` ; bornes 0–10 000 et 1–200. Paramètres inconnus refusés.
Tierlist triée par position puis champion ; builds par catégorie, effectif
décroissant, sélection. La pagination des builds ne tronque pas compétences/achats.

Les métadonnées conservent les dates source/publication, seuil, méthode, couverture
du périmètre et fenêtre calculée. `rank_scope` et `rank_max_age_hours` décrivent le
rang figé à la partie (#80) ; la couverture ajoute la part `UNKNOWN`
(`unknown_rank_rate`) et les écarts partie → observation (`rank_gap_median_hours`,
`rank_gap_max_hours`), `null` pour un instantané antérieur. Les builds incluent les étapes
d'achat (#81 : `starter`, `boots`, `core` ordonné, `item_slot_4..6`) et chaque variante
porte `win_rate_lower_bound` (Wilson 95 %). La réponse builds ajoute `build_stage_method`
et `item_catalog_version` (catalogue #61 joint au patch demandé, `null` sans étapes) ; la
couverture ajoute `item_stage_participations` et `missing_item_catalog_participations`
(`0` pour un instantané antérieur). Couverture et bans ne sont pas ventilés par
rôle/rang. Taux sous seuil `null`, `total` avant pagination. Périmètre absent :
liste vide ; snapshot absent/incompatible : 503. `omitted_build_variants` reste
le compteur **global du snapshot**. Pas de filtre temporel arbitraire : demander
un nouveau calcul au collecteur. Les restrictions Arena/augments #18 sont conservées.

La lecture accepte les instantanés historiques complets et le stockage en morceaux
du collecteur. Tête et morceaux sont lus dans une seule requête cohérente. Pour le
stockage v2, l'index GIN des populations sélectionne les morceaux par section,
patch, plateforme, file, rôle, rang et champion avant de lire leurs entrées JSON.
Un filtre fin reste appliqué aux morceaux contenant plusieurs populations. Une
tierlist ne charge pas les morceaux de builds ou de timelines. Aucun rapport global
dépassant la limite JSONB n'est reconstitué côté PostgreSQL.

La migration `0009` renseigne l'index des morceaux déjà publiés et s'applique au
démarrage du service. Elle réécrit et verrouille la table des morceaux : prévoir
cette opération avant de remettre le service en trafic sur une base volumineuse.
Les rapports historiques en stockage v1 restent lisibles mais ne bénéficient du
filtrage indexé qu'après un nouveau calcul du collecteur. Mesures et non-régressions :
[recette du filtrage des builds](../../docs/recettes/2026-10-02-filtrage-builds.md).

### Profils, historique et quotas

account-v1 résout l'identité actuelle ; aucun annuaire de pseudos historiques.
summoner-v4 et league-v4 complètent le profil, cache 5 minutes/1 024 entrées.
Classement vide réussi : non classé ; panne : erreur, aucun rang fabriqué.
Chaque rang Solo/Flex porte aussi, quand league-v4 les renvoie, `wins`, `losses`,
`hot_streak`, `veteran`, `fresh_blood` et `inactive` (`null` si non classé ou absent ;
jamais déduits). Le peak de saison et le rang de fin de saison précédente ne sont pas
fournis pour un tiers : league-v4 ne les expose pas et ils ne sont pas reconstruits.
Ils ne sont disponibles que pour le compte actif, depuis le client LoL (partie desktop).
Les parties live ne sont pas encore fournies. Le contrat JSON `Profile` a une seule
référence, `packages/shared/src/contracts/profile.json`, relue par les tests de
l'API, du client Rust (`olc-build-client`) et de `@olc/shared`.

Historique : `start=0`, `count=10`, maximum 20, début limité à 10 000.
`next_start` est le prochain index Riot, `null` en fin de liste ou à la borne
locale. `omitted_matches` compte les parties personnalisées ou d'autres plateformes
après transfert. Suivre le curseur même pour une page vide après exclusion.
Aucun pseudo historique ni identité adverse dans les projections. Cache borné
à 4 096 projections de parties. Une nouvelle partie peut déplacer les pages :
dédupliquer par `match_id`. Une panne Riot fait échouer la page entière.

Les transports réels API/collecteur partagent `riot_shared_quota` : fenêtres par
hôte/méthode, compteurs et pauses 429. Réservation validée avant envoi, conservée
après annulation. Tous les processus doivent utiliser cette version et **la même
base** ; les outils tiers ne sont pas coordonnés. Une base représente un produit
Riot ; aucun joueur ni clé dans les états. Les appels en vol non confirmés sont
comptés prudemment pendant 60 s (timeout HTTP réel 15 s).

Priorité : chaque transport déclare sa priorité à la construction (paramètre obligatoire,
pas de valeur par défaut). L'API utilise `Interactive` (plafond complet de la clé) ; le
collecteur utilise `Background` et ne consomme que 80 % de chaque fenêtre (au moins
1 appel), soit 16 par seconde et 80 par 2 minutes avec les limites d'une clé de
développement. Les 20 % restants forment une **réserve** que seul l'interactif peut
consommer : elle réduit l'attente d'une recherche de profil pendant une rafale de
collecte, mais **ne garantit ni réponse immédiate ni absence d'attente**. Limites :

- la réserve est consommable : une fois ses appels utilisés (par l'API elle-même), une
  recherche attend comme les autres, jusqu'à 20 s puis `riot_busy` ;
- un 429 Riot, ou un autre outil qui consomme la même clé hors de cette base, bloque ou
  vide le seau pour tout le monde, réserve comprise ;
- une fenêtre d'un seul appel ne laisse aucune réserve ;
- elle ne s'applique qu'aux processus qui partagent la même base ; le plafond global de
  Riot n'est jamais dépassé et aucune seconde clé n'est utilisée.

### Cache et erreurs

Statiques : `ETag` SHA-256 du contenu, `If-None-Match` et 304 vide. Manifeste :
cache navigateur 60 s, CDN 300 s. Documents : 3 600 s. Une même version peut être
corrigée : pas de directive `immutable`. Configurer le CDN pour respecter ces
en-têtes et cacher uniquement les routes statiques. Images sur les CDN Riot.
Cette commande ne déploie ni CDN ni terminaison TLS.

Erreurs JSON : `{"error":{"code":"…"}}`. Codes : `invalid_request` (400),
`unauthorized` (401), `forbidden` (403, sujet non habilité aux routes RGPD), `not_found` (404), `unavailable` (503), `rate_limited` (429, refus sans attente de quota : les 4 emplacements d'appels Riot de
l'instance sont pleins, ou les 32 emplacements HTTP, ou les 128 emplacements WebSocket
(refus de l'ouverture), ou Riot a lui-même répondu 429, propagé tel quel),
`riot_busy` (503,
aucun créneau de quota Riot obtenu en 20 s : réessayer plus tard, ce n'est pas une panne).
Aucun corps brut Riot, URL sensible ou détail SQL. Le consommateur traduit ces
codes en FR/EN. Durée HTTP maximale 45 s, appel Riot 20 s avec attente de quota ;
maximum 4 appels Riot et 32 requêtes HTTP simultanés par instance.

## WebSocket

`ws://127.0.0.1:3030/v1/ws` en local. Premier message sous 5 s :
`{"type":"authenticate","token":"<jeton d'accès>"}`. Aucune autre commande.
Le serveur émet immédiatement puis à chaque changement :
`{"type":"data.updated","stats_version":null,"static_version":null,"available":false}`.
Versions = dates de publication ; `null` = aucune donnée, `available=false` = base
non vérifiée. Relire REST après notification/reconnexion : pas de rejeu de toutes
les publications intermédiaires. Aucune donnée personnelle sur le flux.

Une surveillance SQL commune toutes les 5 s. Maximum 128 sockets ; frames/messages
8 Kio ; écritures 5 s ; ping 30 s. Fermeture sur expiration JWT, message invalide
ou arrêt (Ctrl+C Windows/macOS, SIGTERM également sous Unix).

Premier client : le cœur Rust du desktop (`crates/build-client`, #125). Il
s'authentifie par le premier message, relit REST via un changement de `revision`
exposé à l'interface, reconnecte avec un repli de 1 s à 60 s et cesse après une
fermeture `1008` (jeton refusé ou expiré), le jeton du desktop étant fixe.

## Validation et sources

`pnpm test` inclut l'API. Définir `OLC_TEST_DATABASE_URL` pour les lecteurs SQL et
quotas : bases jetables, Riot synthétique, aucune clé nécessaire. `pnpm lint` :
typage, format et Clippy. CI Linux avec PostgreSQL 17 ; Windows/macOS sans base.

[Axum 0.8.9](https://docs.rs/axum/0.8.9/axum/),
[JWT 9.3.1](https://docs.rs/jsonwebtoken/9.3.1/jsonwebtoken/),
[Riot ID](https://developer.riotgames.com/docs/lol#summoner-names-to-riot-ids),
[API Riot](https://developer.riotgames.com/apis),
[quotas](https://developer.riotgames.com/docs/portal#web-apis_rate-limiting),
[politique Riot](https://developer.riotgames.com/docs/lol#game-policy).

Recette réelle macOS : [bilan du 1er octobre 2026](../../docs/recettes/2026-10-01-api-interne.md).
