# Recette #63 — imports au prépick, partie client

Date : 02/10/2026. Branche `codex/auto-import-client`, basée sur le front desktop
`ee2d73b` (#66, correctif de rattrapage du compte et CI inclus). Aucun overlay ni changement des sorts.

## Vérifications réalisées

- Rouge puis vert : projection gameId/file ; refus d'un autre contexte LCU ;
  scénario d'échange aller-retour avec une écriture encore en vol ; prépick en
  personnalisée ; identité locale avant gameId ; dédoublonnage au verrouillage ;
  lecture de draft momentanément absente sans réarmer l'import.
- `pnpm test` : succès, 177 tests TypeScript et suites Rust au vert, dont
  107 tests du connecteur. Les recettes Rust longues déjà marquées `ignored`
  ne sont pas activées. Les tests PostgreSQL conditionnels ne sont pas activés
  par cette commande (aucune modification du backend dans ce lot).
- `pnpm lint` : succès, typage, formatage et Clippy sans avertissement.
- Compilation et lancement natifs macOS : succès. API locale et transport de
  développement démarrés avec un jeton temporaire de quatre heures dans
  l'environnement, sans secret serveur transmis au desktop.
- API réelle locale : lecture de 88 variantes pour Bard, EUW1, queue 420,
  rôle UTILITY, rang ALL, patch 16.19. Le sélecteur réel et le catalogue embarqué
  acceptent une page de runes (4 parties, 3 victoires) et un inventaire final
  (1 partie, 1 victoire) au seuil 1. Le taux de 100 % sur cet unique inventaire
  est uniquement une observation ; aucun ordre d'achat ni confiance statistique
  n'en est déduit. La fixture temporaire n'est pas intégrée au produit.
- Déplacement dans les paramètres : test de visibilité rouge puis vert ;
  aperçu navigateur 1280 × 720, panneau présent dans Réglages et absent de
  l'accueil et de la draft. Les paramètres défilent dans la zone principale.
  Le composant conserve une position React stable et ses effets restent actifs
  hors paramètres ; aucun remontage du moteur pendant la navigation.
  Imports désactivés dans l'aperçu navigateur, comme prévu.
- Après le retour utilisateur, lecture seule du client réel via Rust : salon
  personnalisé Faille, file 3100, mode CLASSIC, gameId 0 ; cinq pages éditables,
  dont exactement une réservée à l'app. Ces conditions étaient exclues par
  l'ancien déclencheur (verrouillage, poste attribué, gameId positif, file standard).
- Correctif : prépick pris en charge, poste choisi explicitement en personnalisée,
  source statistique Solo/Duo affichée, identité locale de draft sans attendre
  gameId. Test Bard → Ahri → Bard : trois PUT sur la même page réservée, aucun
  POST/DELETE ni écriture de sorts. Vérification du panneau dans l'aperçu navigateur.
- Revue stricte indépendante : échange aller-retour et lecture absente corrigés
  puis revérifiés ; aucun autre blocage P1/P2 relevé. Verdict **OK avec réserves** :
  recette réelle des deux OS et réapplication possible après reconnexion complète.
- Auto-revue : population exacte, absence de fallback implicite, catégories
  indépendantes, protection des modifications manuelles, invalidation des
  réponses anciennes et absence de relance automatique après erreur.
- Conformité §2 : activation explicite par réglage, écritures limitées au client
  en sélection, aucun appel d'import des sorts, aucune action en partie, aucune
  donnée adverse cachée ni injection ; mention Riot existante conservée.
  [Politique et documentation Riot](https://developer.riotgames.com/docs/lol).

## Limites de cette recette

Aucun champion n'a été verrouillé pour le joueur par l'agent, et aucune écriture
n'a été déclenchée dans son client durant la recette. Le succès réel après
activation doit donc être confirmé lors de sa prochaine draft. Windows n'est
pas exécuté sur cette machine ; le code commun et les chemins existants sont
conservés, recette native Windows et vraie draft macOS à réaliser.
Une reconnexion LCU complète crée une nouvelle génération de draft et peut
réappliquer les imports activés ; une simple lecture absente conserve l'identité.

Le service de statistiques lit les agrégats déjà publiés. Le recalcul périodique
reste géré par le collecteur/agrégateur existant ; lancer cette version du desktop
ne crée pas une nouvelle tâche planifiée de calcul ni de collecte.
