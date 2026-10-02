# Recette Champions desktop — 2 octobre 2026

Périmètre #13, conformément au [plan](../plan-champions-desktop.md). Code local non
commité ; aucune PR ni publication du site. Les recettes League de la passe précédente
restent différées à la demande de Louison.

## Résultat

173 cartes illustrées, noms FR/EN, filtre par classe officielle, tri A–Z/Z–A,
fiche compétences/builds/catalogue, recherche globale au clavier. État Champions
indépendant de la préparation en draft. Aucun import ni réglage Flash dans la fiche.
Index léger généré à partir des fiches locales 16.19.1 ; génération reproductible.

## Contrôles exécutés

- Tests rouges puis verts : noms/accents/ponctuation/classes/tri ; conservation du
  contexte et navigation automatique ; réponse tardive/erreur/réessai ; lecture seule ;
  clavier des onglets.
- `pnpm test` : **498 tests passent** (139 desktop, 10 shared, 349 Rust) ; deux recettes
  explicites sur sources publiques restent ignorées. Aucun test en partie lancé.
- `pnpm lint` : succès, code de sortie 0 (TS, fmt, Clippy).
- `pnpm --filter @olc/desktop build` : succès. Avertissement Vite de taille de bundle
  présent ; optimisation du compagnon/chargement global hors périmètre.
- `pnpm --filter @olc/desktop tauri build --debug --bundles app` : succès, bundle Mac
  68,33 MiB. Ce build ne remplace ni la recette native ni un installeur signé/notarisé.
- `git diff --check` : succès. Auto-revue limitée aux fichiers du lot, comparés à la
  baseline conservée ; revue indépendante : **OK après correction de deux P2 clavier**.

## Parcours navigateur observés

- 960×600 : cartes + fiche Ahri, pas de débordement global (scrollWidth=960,
  scrollHeight=600), défilement local des deux panneaux.
- 1280×800 : pas de débordement global ; ouverture Garen au clavier ; réglages puis
  retour avec le même champion et scrollTop **1384,5** avant/après.
- Ahri + recherche « ah » + classe Mage, aller-retour réglages, passage FR→EN :
  recherche, classe et sélection conservées ; sombre et clair inspectés.
- Recherche globale « vi » : Vi proposé en premier ; Entrée ouvre sa fiche.
- Défaut de recherche reproduit avant correction : résultat actif sous la fenêtre
  (bottom 851,5, panneau 482,5). Après correction, résultat actif défilé dans le panneau.
- Onglets : ArrowRight depuis Compétences sélectionne et donne le focus à Builds ;
  logique Home/End et bouclage couverte par test.
- Catalogue : arbres complets et fragments, objets et composants consultables.
  Fiche détaillée du passif d’Ahri ouverte ; texte officiel lisible, sans HTML brut.
- Builds sans pont natif : état explicite « statistiques accessibles dans l’application
  desktop », aucun faux résultat. Réponses tardives et réessai testés unitairement.
- Panneaux alimentés vérifiés séparément avec une **fixture signalée comme fictive** :
  arbres complets, runes sélectionnées, séquence d’achats avec doublon et composant,
  sorts d’invocateur ; 0 bouton d’import et 0 préférence Flash. Ce harnais est retiré
  des sources et n’est pas embarqué. Il ne valide pas l’API de production.
- Aucun message d’erreur console dans la vue finale.

## Livraison et limites

Bundle : `target/debug/bundle/macos/Open LoL Companion.app`.
SHA256 de `Contents/MacOS/olc-desktop` :
`47049c1293e7028ff084ba4d98e8176d7edb59bb19adc8fc34cf9c622b5dc891`.
Le bundle antérieur laissé dans le dossier Codex `sal/outputs` n’est pas remplacé.
Cette nouvelle version est compilée, pas recettée avec League.

Preuves, captures, baseline et logs : `work/champions-2026-10-02/`.
Aperçu local laissé actif : `http://127.0.0.1:1421/`, page Champions dans l’onglet livré.
La surcharge de taille du navigateur est réinitialisée. Les deux fichiers de recette
ont été déplacés dans le dossier de preuves ; le cache de relance temporaire a été
supprimé. Le premier build avait attendu sur un fichier Cargo iCloud `dataless`, puis
a terminé avec succès ; la seconde compilation devenue inutile a été arrêtée.

Aucune nouvelle commande Rust, aucun type IPC ni secret. Données officielles statiques
et statistiques agrégées via le contrat existant ; aucune décision de jeu automatisée.
[Politique Riot consultée](https://developer.riotgames.com/policies/general).

Restent hors lot : profils de joueurs, maîtrise, matchs récents, conseil personnalisé,
matchups et builds pro. La grille filtre des **classes** ; le poste filtre seulement
les builds. Les variantes temporaires de build ne sont pas mémorisées au démontage.
Recette native Windows/macOS et connexion aux statistiques de production à reprendre
séparément ; le site #20 reste au périmètre de Matthieu.
