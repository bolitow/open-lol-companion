# Compagnon — première intégration rouge (#1)

Le composant `src/companion/FlameCompanion.tsx` est partagé entre l’accueil réel et la maquette. Il remplace la marque décorative de la carte de bienvenue et occupe un emplacement réservé dans le bandeau du prototype. Aucune donnée fictive supplémentaire ni interaction avec le jeu.

## Comportement

Apparition et salut une seule fois par session de page, après le premier chargement visible réussi. Revenir à l’accueil ne rejoue pas le salut. Repos discret, anticipation de 0,24 s au clic, rotation de 1,25 s, visage diablotin pendant 2,4 s, puis retour au calme en 1,4 s. Le salut dure 1,2 s. Les clics répétés ne relancent pas une transformation en cours. L’option Animations et la préférence système de réduction des mouvements donnent une pose statique ; un clic peut encore changer l’expression sans rotation. Le rendu est suspendu hors écran, dans un onglet masqué et lorsque la maquette le masque. Au démontage, animation, observateurs et ressources GPU sont libérés. Un pictogramme remplace le modèle en cas d’échec WebGL ou de chargement.

## Feu

La couleur et la turbulence reprennent `prototype/fireShader.ts`. Le modèle est d’abord dessiné sur une texture transparente ; son alpha animé alimente des langues ascendantes irrégulières, concentrées sur la partie haute. Il n’y a plus de plan de flammes derrière le personnage. Les effets suivent ainsi les gestes et le retournement. Le contour lumineux uniforme a été supprimé ; le feu utilise un alpha droit pour éviter une double atténuation.

Le modèle local `public/companion/flame.glb` provient de l’essai V4, exporté et allégé par `work/flame-mascot-probe/v6/export.py` : dix clips, deux visages, environ1,12Mo. La version V5 de VFX n’est pas embarquée. Dépendances : Three.js et types0.180.0 ; chargement différé du moteur et de l’asset.

## Validation et limites

- Tests d’état écrits en échec puis verts : parcours complet, clic répété, masquage, mouvement réduit et couleur progressive.
- Contrôle navigateur macOS de l’accueil, des préférences et de la navigation ; rendu natif WebKit macOS observé dans une instance de développement isolée.
- Tests/lint du monorepo et build Vite vérifiés dans la recette de cette itération.
- Windows/WebView2 reste à vérifier. Aucune validation artistique définitive ni mesure de budgetGPU n’est revendiquée.
- Décision de Louison : valider entièrement la version rouge sur thème sombre avant de décliner le même compagnon en bleu. La palette est donc actuellement rouge, quel que soit le thème technique de l’app. Les réactions aux phases LCU ne sont pas encore raccordées.

### Itération V8

Sept tests d’état couvrent aussi l’accueil de session et l’anticipation. Contrôle navigateur de la navigation, de la transformation et du retour au calme. La vérification native macOS mentionnée ci-dessus concerne V7 ; Windows et les mesures GPU restent à réaliser pour V8.
