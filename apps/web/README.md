# apps/web

Site Next.js du [§9 du cahier des charges](../../docs/cahier-des-charges.md) (#20) :
tierlist, page champion, profil joueur et recherche globale Ctrl+K, en français
(`/fr/…`) et en anglais (`/en/…`). Données lues dans l'[API interne](../../services/api/README.md)
publiée par #19 ; aucun calcul statistique côté site.

## Démarrage

Node 22 et l'API lancée (`cargo run -p olc-api -- serve`, voir son README).
Créer `apps/web/.env.local` à partir de [`.env.example`](.env.example) :

| Variable | Usage |
| --- | --- |
| `OLC_API_URL` | Origine de l'API, défaut `http://127.0.0.1:3030` |
| `OLC_API_TOKEN` | Jeton de lecture émis **sur le serveur** : `cargo run -q -p olc-api -- token --subject web --ttl 86400` |

```sh
pnpm --filter @olc/web dev     # http://localhost:3000/fr/tierlist
pnpm --filter @olc/web build   # build de production
pnpm --filter @olc/web test    # tests Vitest de la logique (filtres, Riot ID, recherche…)
```

Sans jeton, les pages affichent « accès à l'API non configuré » ; sans statiques
publiés, la tierlist l'indique au lieu d'inventer un patch.

## Secrets et appels réseau

- Toutes les requêtes à l'API partent du serveur Next.js (`src/lib/server.ts`,
  marqué `server-only`). Le jeton n'est jamais préfixé `NEXT_PUBLIC_`, n'apparaît
  ni dans le HTML, ni dans une URL, ni dans un journal.
- Le navigateur ne contacte que le site et le CDN Data Dragon de Riot (images).
  Aucune origine CORS n'est donc à ouvrir dans `OLC_API_ALLOWED_ORIGINS`.
- Le jeton expire au plus tard après 24 h (limite de `olc-api token`) : il doit être
  renouvelé par l'hébergement tant qu'aucun mécanisme de rotation n'est décidé.

## Pages

| Route | Données |
| --- | --- |
| `/{fr,en}/tierlist?role=&rank=&platform=&queue=&patch=` | `/v1/tierlist` : tier, position, victoires, sélection, bans, effectif |
| `/{fr,en}/champions/{slug}?…` | `/v1/builds/{id}` : résumé, runes, sorts, ordre des compétences, objets finaux, objets les plus fréquents, totem |
| `/{fr,en}/profile/{plateforme}/{nom}/{tag}?start=` | `/v1/profiles/…` et `/matches` : identité, niveau, Solo/Flex, historique paginé |

- Filtres validés comme l'API (`services/api/src/query.rs`) : une valeur invalide
  retombe sur la valeur par défaut (EUW1, Solo/Duo, Milieu, tous les rangs, patch
  de la version statique en ligne). Un seul rôle à la fois : les populations ne
  sont jamais additionnées.
- Les taux sous le seuil restent `—` ; seuil, méthode du tier, définition du taux
  de sélection et date de publication sont affichés sous chaque tableau. L'API
  publie les taux en points de pourcentage (0–100) ; les dates sont affichées en UTC.
- Le slug champion est l'identifiant Data Dragon en minuscules (`monkeyking`).
- Recherche Ctrl+K / Cmd+K : champions du catalogue statique, ou Riot ID complet
  (`Nom#TAG`) saisi par le joueur avec sa région. Aucune recherche d'équipe ni de
  joueur pro : l'API ne les fournit pas.
- Les profils ne sont pas indexés par les moteurs (`noindex`) et l'identifiant
  `puuid` n'est pas affiché.

## Conformité Riot

Statistiques agrégées sans joueur précis et profils publics consultés sur saisie
manuelle (Game Policy). Aucun MMR, aucune donnée d'augment, aucune file Arena ;
mention légale Riot en pied de chaque page.

## Non fournis par l'API à ce jour

Exigences du §9 et de #20 encore ouvertes, faute de donnée publiée par l'API :

- profil : statistiques par champion, pic de rang, suivi des LP, parties en direct ;
- page champion : builds pro, contres, synergies ;
- tierlist : filtre de période (seul le patch est proposé) ;
- transferts de région d'un joueur (un profil est lu sur la plateforme saisie) ;
- matchups, leaderboards, recherche d'équipe et de joueur pro.
