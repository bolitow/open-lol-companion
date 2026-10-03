# Recette Collection et lecteur SkinSpotlights — 3 octobre 2026

Ticket : #47. Branche de livraison : `codex/collection-video-reliability`.

## Périmètre

Cette livraison rassemble la Collection, son catalogue de 58 vidéos pour 30 champions (55 segmentées, 441 passages), ses générateurs et le lecteur natif. Elle inclut les prérequis UI/compétences/draft déjà développés sur la branche locale, rapprochés de `main` avec conservation des imports de sorts et de la préférence Flash. Les retouches Accueil/Profil/Amis encore non commitées dans le répertoire principal sont exclues et préservées.

## Vérifications automatiques

- `pnpm test` : 995 réussis (508 Rust, 477 interface, 10 types partagés), deux ignorés préexistants.
- `python3 -m unittest discover -s scripts -p 'test_*.py'` : 26 réussis.
- `pnpm lint` : réussi ; compilation Tauri macOS debug : réussie.
- Régressions rouge/vert : fermeture après ouverture externe réussie, suivi des tentatives de chargement et messages FR/EN. Les événements d’anciennes tentatives ne peuvent pas écraser la tentative courante.
- Revue indépendante du lecteur : OK avec réserves de recette. Le signal « chargé » concerne le document, pas l’état de lecture YouTube.

## Recette native macOS

- Collection → Ahri de minuit → lecteur : ouverture et boutons de passages présents ; Emotes sélectionne les bornes 0:27–0:58.
- Réessayer recrée le lecteur en conservant le passage. Une lecture avec images de jeu à 0:30 a été observée pendant le diagnostic, **mais un écran noir a de nouveau été reproduit dans le binaire final**. L’anomalie reste intermittente et non résolue ; la relance n’est pas une garantie de récupération.
- Détacher puis rattacher : fenêtre native ouverte puis refermée, titre et passage Emotes conservés. La continuité temporelle d’une vidéo en lecture n’est pas validée par ce cas noir.
- Ouvrir sur YouTube : lecteur intégré fermé, retour à la fiche du skin, navigateur ouvert sur la vidéo attendue avec `t=27s`. Onglet de test fermé ensuite.
- Réglages après intégration : imports runes/objets/sorts présents et désactivés comme avant ; position Flash D conservée. Contrôle en lecture seule, aucun import lancé ni préférence changée.

## Réserves avant clôture

- Le ticket #47 reste ouvert. L’écran noir Ahri de minuit demande encore un diagnostic média ; le suivi du document et le repli externe ne remplacent pas les événements de l’API IFrame YouTube.
- Le poste Windows distant était indisponible lors de deux tentatives. Aucune recette native Windows n’est revendiquée. Les builds/tests CI multi-OS restent distincts d’un essai avec le client League réel.
- Les 58 vidéos n’ont pas toutes été lues manuellement. Les 116 candidats précédemment bloqués par HTTP 429 restent en attente, sans nouvelles requêtes massives.
- Journaux et captures locaux : `work/spotlight-reliability-47/` dans le répertoire de travail principal ; ils ne sont pas embarqués dans la PR.
