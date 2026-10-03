# Recette macOS — imports au prépick (#63), 3 octobre 2026

## Périmètre et plan avant code

Après #22 et #23, vérifier les imports réels au prépick : runes et set d’objets,
réutilisation de la page réservée, absence de destruction des pages personnelles,
retour A → B → A et relais vers la partie. Aucun clic ni touche automatisé dans
League : le joueur effectue les sélections, le probe ne fait que des GET LCU.

Pendant cette recette, le joueur a corrigé son premier retour : **seules les
runes sont confirmées visuellement**. Il a ensuite demandé explicitement d’ajouter
l’import des **sorts les plus joués avec respect de Flash D/F**. Cette décision
remplace l’exclusion des sorts dans le ticket d’origine. Au moment des essais ci-dessous, le ticket GitHub était inchangé et cette
passe était locale, avant la demande de livraison en MR.

Plan de l’extension : sélectionner `summoner_spells` avec population/seuil
existants ; ajouter une option indépendante ; partager la préférence explicite
`olc.flash-slot` avec l’éditeur manuel ; réutiliser le moteur #15 après les gardes
#63 ; séparer les trois verrous/caches et relire la paire dans le même contexte.
Tests avant implémentation, puis vérifications complètes et reconstruction Mac.

## Environnement et première séquence réelle

- Mac mini M4 Pro, 24 Go, arm64 ; macOS 27.0 (26A428).
- League 16.19.8230722 ; personnalisée Faille identifiée par gameflow, carte 11,
  CLASSIC, file 3100 ; poste manuel Mid, statistiques Solo/Duo explicitement utilisées.
- Premier binaire : 0.1.0 debug construit sur `31c8d791b0ad9410b860c1690ac24e52d47c5d10`,
  SHA-256 `1634ce295deceb21bd65bb0147f9a2bf0b2f4584997473af4258324fab46534b`.
- Paramètres initiaux : runes et objets activés, seuil 100, poste personnalisé
  absent. Seuil abaissé à 1 et poste Mid pour la recette ; cela ne prouve pas
  qu’une variante atteigne le seuil de production 100.
- L’app s’était relancée sans configuration du service statistique dans son
  environnement. Une relance avec l’URL locale et un jeton temporaire a restauré
  le chargement. Cause de la première relance inconnue. Aucun secret stocké dans
  ces preuves ; ce rétablissement de session ne constitue pas une configuration
  publique durable de l’app.

[Relevé réduit et anonymisé](assets/2026-10-03-imports-macos-avant-sorts.json) :
lectures successives environ chaque seconde, conservation des changements d’état.
Les réponses gameflow/pages/sets/draft **ne sont pas atomiques** ; une ligne lors
d’un changement rapide ne permet pas de conclure à un import pour le mauvais
champion. L’alias de page est synthétique, uniquement valable dans ce relevé.

| Contrôle | Résultat et portée |
| --- | --- |
| Ahri au prépick après reconnexion statistique | Le joueur confirme les runes. À 09:49:47 Europe/Paris, relecture de la page 8100/8200 avec neuf runes ; pas de verrouillage requis. |
| Objets Ahri | Set enregistré côté client à la même période, associé à Ahri (103) et carte 11. La confirmation dans la boutique reste distincte et non obtenue à ce stade. Aucun achat automatique. |
| Page réservée | Un seul alias de page pendant les changements observés, notamment Ahri → autre champion → Ahri. Aucun comptage d’écritures réseau déduit de ce relevé. |
| Pages personnelles | Quatre pages avant/après ; contenu et propriétés projetées inchangés dans toutes les lectures disponibles. |
| Sets personnels | Aucun set personnel au départ : leur conservation en situation réelle avec un set non vide n’est pas prouvée. Les tests mockés couvrent ce cas. |
| D/F avant extension | Plusieurs changements observés pendant les manipulations du joueur ; le probe ne permet pas d’attribuer leur cause. Aucun verdict global « D/F inchangés » n’est tiré de cette séquence. |
| Entrée en partie / boutique | Non observées dans cette séquence ; retour au salon à 09:50:25. |

## Extension sorts et vérification automatisée

- TDD : huit tests TypeScript échouaient avant raccordement ; trois scénarios
  Rust refusaient encore la variante IPC `spells`. Après implémentation, sélection
  par popularité, seuil, population, Flash D/F, paire sans Flash et rejet des
  réponses statistiques anciennes sont verts.
- Option absente des anciens réglages = désactivée. Si la meilleure paire valide
  contient Flash sans préférence explicite, seuls les sorts attendent D/F.
- Préférence partagée et persistance testées ; stockage inaccessible ne choisit
  aucune position implicite et signale un échec.
- Gardes de champion/génération avant PATCH et relecture du même contexte après
  écriture ; seule la paire normalisée concordante est confirmée. Les payloads
  mockés contiennent exactement les deux sorts, aucun champ de skin.
- Cache et verrou propres à chaque catégorie ; le verrou sorts est partagé avec
  l’import manuel. Préservation d’une modification dans LoL au verrouillage.
- Dès l’envoi, un changement D/F concerne les prochains envois : test F en cours
  → choix D → succès F unique → nouveau champion importé avec D. Aucun second
  PATCH automatique après une réponse incertaine ou un import réussi.
- `pnpm test` : **264 tests TypeScript et 437 tests Rust réussis**, deux recettes
  catalogue explicitement ignorées. `OLC_TEST_DATABASE_URL` absent : les chemins PostgreSQL conditionnels
  ne sont pas validés dans cette exécution. Aucun résultat local n’est présenté
  comme une validation CI sur trois OS.
- `pnpm lint` : réussi, typage TypeScript, fmt et Clippy à zéro.
- Auto-revue : contrat Rust/TypeScript aligné, absence de nouvelles dépendances,
  d’endpoint inventé, de secret, de commande d’achat ou de sélection de champion.
- Revue stricte indépendante : **OK**, après explicitation du changement D/F
  pendant un envoi. Cette revue ne vaut pas recette du nouveau binaire réel.

## Contrôles réels restant à consigner

Le binaire incluant les sorts a été construit avec
`CARGO_TARGET_DIR=/Users/bolito/dev/open-lol-companion/target pnpm build:desktop --debug --bundles app`
et relancé avec sa configuration statistique. SHA-256 :
`ba7f8ac11b82cb61dc590ae05cf2abfbf5f1f78cafff1f9760bb914ccec9f6d0`.
Il correspond à la base `31c8d79` augmentée du diff local de cette recette,
et non à un nouveau commit. L’interface réelle conserve les options runes/objets et propose les sorts
initialement décochés. L’option sorts a été activée pour l’essai en conservant
la préférence **Flash sur F** déjà mémorisée. Le joueur confirme : « Oui, les
sorts sont importés avec Flash sur F ». Le relevé LCU montre plusieurs changements
de champions et de l’autre sort, Flash restant sur F ; les lectures non atomiques
ne sont pas utilisées pour attribuer une paire précise à chaque changement rapide.

Le joueur confirme également que son autre sort, modifié manuellement après
l’import puis conservé cinq secondes, reste inchangé au verrouillage.

Le joueur confirme ensuite le set « Open LoL Companion » dans les ensembles
d’objets de la boutique, après lancement de la personnalisée. Le passage gameflow
à `InProgress` est aussi observé. Cela confirme l’existence du set dans la boutique,
pas son classement exact en première position ni un ordre d’achat optimal.
Le contenu de build consultatif dans le panneau en partie n’a pas été confirmé
séparément lors de cette séquence. Flash sur D et paire sans Flash sont
couverts automatiquement ; pas de validation réelle de ces deux variantes lors
de cette passe à ce stade.
Les recettes de page pleine, indisponibilité API réelle, reconnexion et conservation
d’un set personnel non vide ne sont pas déduites des seuls tests automatisés.
La recette Windows reste à effectuer avec Louison : **#63 n’est pas clôturé**.


## État final de la passe locale

- [Relevé après activation des sorts](assets/2026-10-03-imports-macos-avec-sorts.json) :
  projection de 21 changements d’état, terminée à l’entrée en partie. Le probe
  a été arrêté après la confirmation humaine du set en boutique.
- L’interface a ensuite signalé « Hors partie ». La disparition complète du
  contenu de build dans l’overlay n’a pas fait l’objet d’une nouvelle confirmation.
- Ajustement visuel final : boutons D/F distincts et préférence active visible.
  Le binaire final `f4ad464b64cb8b766cb20ac225c230467545d4860335561b4d05979d8fbba2c8`
  diffère du binaire de test réel ci-dessus uniquement par cette présentation.
  Il a été relancé ; l’interface et ses paramètres ont été vérifiés.
- [Écran des réglages finaux](assets/2026-10-03-imports-reglages-macos.png) : seuil
  restauré à **100**, poste en personnalisée restauré à **Choisir un poste**,
  runes/objets conservés activés, **sorts activés et Flash sur F**. Les imports en
  personnalisée exigent donc de choisir à nouveau un poste ; le seuil de recette
  1 n’est pas conservé.
- Pendant la restauration via l’outil de contrôle, des actions clavier ont
  coïncidé avec la fermeture du compagnon ; la cause n’est pas établie. La
  restauration a finalement réussi par sélection de texte et collage, sans
  raccourci clavier. La session finale a sa configuration statistique rétablie.
- Phase C réexécutée après le changement visuel : `pnpm test` et `pnpm lint`
  réussis, mêmes comptes et limites ; build Mac réussi ; revue indépendante OK.
  Ces essais précèdent la demande ultérieure de commit, push, MR et notification
  de Louison ; ils n’avaient alors produit aucune écriture sur GitHub.
