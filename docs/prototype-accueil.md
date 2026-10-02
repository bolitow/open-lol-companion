# Prototype interactif de l’accueil — ticket #4

## But et périmètre

Éprouver la direction graphique en mouvement avant de la généraliser : thème sombre natif avec accents rouges, thème clair avec accents bleus, panneaux compacts, profondeur discrète et recherche centrale. La composition conserve profil et session côte à côte, historique en dessous, amis et progression dans deux panneaux de même hauteur à droite.

Cette entrée React séparée utilise uniquement des données fictives. L’écran LCU existant reste à `/`. Aucune connexion au jeu, capture, analyse réelle, recommandation calculée ou fenêtre d’overlay n’est implémentée ici. Le build Vite inclut les deux entrées, sans importer le prototype dans l’écran LCU.

## Lancer

Depuis la racine du dépôt, après `pnpm install` :

```bash
pnpm --filter @olc/desktop dev --host 127.0.0.1 --port 1421
```

Ouvrir **http://127.0.0.1:1421/prototype.html**. Le port est distinct du serveur Tauri habituel. Aucun client LoL n’est nécessaire.

## Parcours à essayer

- Recherche : joueurs, champions et accès rapides ; accents ignorés, filtres, flèches et Entrée, `Ctrl+K` ou `Cmd+K`. Le résultat actif reste visible.
- Ahri → Progression → Retour · Ahri : le champion sélectionné reste ouvert. Les autres destinations sont des aperçus de parcours, pas des pages complètes.
- Historique : filtres, défilement interne, détails d’une partie puis retour. Une actualisation conserve le défilement et le focus du bouton.
- Amis : défilement et épinglage. Progression : choix d’un objectif fictif.
- Réglages : thèmes, français/anglais, animations. Ces trois préférences seulement sont enregistrées dans le navigateur.
- Scénarios : fermeture du client simulé en conservant le compte, changement de compte avec statistiques séparées, chargement/vide/erreur et nouvelle tentative.
- Clips : interrupteur de démonstration avec notification explicite ; aucun enregistrement ne démarre.

Les données de session et les interactions se réinitialisent au rechargement : le maintien du compte lors de la fermeture du client est simulé pendant la session de la page.

## Mouvement

L’ouverture comporte trois profondeurs coordonnées. Un shader WebGL original consume une matière texturée depuis le centre, avec une frontière turbulente, une zone carbonisée, un cœur clair et plusieurs halos. Palette rouge/orange/or en sombre, bleu/violet/cyan en clair. Une seconde vague commence 325 ms derrière, dans les interstices et sous les panneaux. Elle éclaire brièvement leurs contours et l’illustration de session.

La première couche disparaît en 1,875 seconde ; les braises s’éteignent à 2,75 secondes. Après retour de Louison, le rythme est ralenti uniformément de 20 % (durée augmentée de 25 %), turbulences comprises, pour mieux lire l’effet. Sur la seule carte de session, un reflet diagonal traverse la matière sous le texte entre 0,75 et 2,625 secondes. Deux bords altérés, des braises discontinues et une chaleur locale persistent ensuite. Cette tranche remplace les trois anciens angles lumineux extérieurs ; elle reste limitée à la session pour validation visuelle avant généralisation. Les marques sont découpées dans les limites de la carte et suivent automatiquement sa taille. Elles restent immobiles ; une ambiance de particules distincte peut ensuite démarrer.

Le shader est limité à 1,1 million de pixels et n’est plus calculé une fois sa couche invisible. Canvas, programme, buffers, shaders et contexte GPU sont libérés à la fin. Un clic, une touche, un défilement de la page ou un redimensionnement termine immédiatement la séquence. Si WebGL, sa précision ou la compilation échouent, l’interface et ses traces statiques restent accessibles. Une perte du contexte interrompt aussi l’effet.

« Rejouer l’ouverture » permet de comparer les thèmes. L’option Animations et `prefers-reduced-motion` désactivent l’ouverture, les particules et les transitions, en conservant les foyers lumineux et les traces statiques. Le budget de pixels ne prouve pas une performance native : fluidité, qualité artistique et confort restent à valider avec Louison sur les machines cibles.

## Validation initiale du 30 septembre 2026 (première version)

- TDD : sept tests du modèle, échec observé avant implémentation puis réussite (préférences, comptes, filtres et recherche).
- `pnpm test` : 12 tests TypeScript et 19 tests Rust réussis.
- `pnpm lint` : typage, format Rust et Clippy réussis.
- `pnpm --filter @olc/desktop build` : deux entrées construites.
- Essais navigateur sur macOS : navigation clavier de la recherche, retour au champion, conservation du filtre et du défilement, déconnexion, changement de compte, état vide, erreur et nouvelle tentative, conservation des préférences après rechargement et désactivation des animations.
- Contrôle visuel : 1440 × 900, 1366 × 768 et fenêtre étroite 390 × 844, sans débordement horizontal observé. Panneaux amis et progression de hauteur égale ; ouvertures rouge et bleue observées à une image intermédiaire, canvas supprimé après la séquence. Console du navigateur sans erreur ni avertissement relevé.
- Revue stricte indépendante : corrections du défilement clavier et de l’actualisation vérifiées, aucun P1/P2 restant ; verdict OK avec réserves de validation native et artistique.
- Les essais natifs Tauri/WebView et Windows restent à faire. Aucun comportement système n’a été ajouté. La conformité d’une future fonctionnalité connectée devra être vérifiée à sa propre implémentation.

## Premier essai du 1er octobre 2026 (remplacé pour l’ouverture)

Louison confirme que l’ancienne ouverture semblait posée au-dessus de l’application et accepte de renforcer la hiérarchie, la compacité et l’identité. La nouvelle version applique l’ouverture aux panneaux eux-mêmes. Le grand titre introductif est remplacé par un en-tête compact ; la session illustrée et l’objectif actif sont mis en avant, l’historique gagne de la place, la liste d’amis reste plus discrète. Sa hauteur reste identique à celle du panneau de progression. Les résultats récents conservent coches et croix en plus des couleurs.

- Cinq tests supplémentaires du front de révélation : échec observé avant implémentation, puis réussite. Vérification des bornes temporelles et du dégagement complet des surfaces sur trois proportions.
- `pnpm test` : 17 tests TypeScript + 19 Rust réussis ; `pnpm lint` réussi.
- Navigateur macOS : rendu sombre à 1440 × 900 et clair à 1366 × 768 inspectés ; étape intermédiaire rouge observée directement dans les cinq panneaux. Après la séquence et après un clic : zéro canvas restant, aucun découpage résiduel. Mode réduit : rejouer désactivé. Parcours Ahri → Progression → Ahri conservé.
- Contrôle complémentaire : ouverture bleue observée, fenêtre 390 × 844 sans débordement horizontal, interruption au redimensionnement sans canvas ni découpage résiduel, état déconnecté lisible et compte conservé, nouveaux libellés anglais vérifiés. Console sans erreur ni avertissement relevé.
- Build Vite réussi ; contre-revue indépendante finale OK, aucun P1/P2 restant identifié après restauration des coches/croix.
- La validation artistique et les essais natifs Windows/Tauri restent à effectuer. Les captures de la section précédente concernent la première version, pas la nouvelle ouverture.

## Deuxième essai du 1er octobre 2026 (ouverture remplacée)

Louison demande une première animation qui part du centre au-dessus de l’app, suivie d’une seconde décalée qui circule entre les panneaux. Il approuve aussi le travail de matière et du module de progression, avec la disposition actuelle conservée.

- Deux couches décrites dans « Mouvement » ; fin automatique et interruptions conservent l’accès à l’interface.
- Surfaces translucides, reflets discrets et éclairage temporaire lié au passage de la seconde vague ; aucun nouvel asset ni dépendance.
- Retrait des indices décoratifs 01/02/03. L’objectif actif montre les cinq mesures fictives, leur trajectoire et une cible en pointillés. Les autres objectifs restent accessibles par défilement. Au changement d’objectif, la sélection reste visible ; format du graphe réduit sur écran de bureau de faible hauteur.
- Une description associée au bouton expose aux technologies d’assistance les cinq valeurs, l’unité, la cible et la nature fictive des données, en FR/EN.
- Six tests de la nouvelle ouverture écrits en rouge puis verts : décalage des deux couches, fin bornée, dégagement des coins sur trois ratios, fente initiale et chemin à l’extérieur des panneaux. Ils remplacent les cinq tests de l’ouverture précédente.
- Validation navigateur : étapes intermédiaires des deux couches en sombre et clair, nettoyage naturel et par clic/redimensionnement, option de réduction des mouvements, sélection du dernier objectif visible et absence de débordement horizontal à 390 px.

Validation finale de cet essai : `pnpm test` (18 tests TypeScript + 19 Rust), `pnpm lint` et build Vite réussis. Contre-revue indépendante : aucun P1/P2 restant identifié. Contrôle du dernier objectif à 1366 × 768 : carte entière visible (158 px dans une zone de 176 px), description FR/EN relue dans le DOM. Parcours Ahri → Progression → Ahri revérifié ; console sans erreur ni avertissement relevé.

Cette version reste un essai artistique ; la fluidité native et les performances Windows/Tauri ne sont pas validées par ces tests navigateur.

## Troisième essai du 1er octobre 2026 (résidus remplacés)

Louison demande une combustion plus organique, des glows de différentes couleurs et des traces de brûlure persistantes. Remplacement du contour Canvas2D au premier plan par le shader décrit dans « Mouvement ». La disposition et les parcours de l’accueil restent conservés. Aucun asset ni dépendance supplémentaire ; aucune authentification web ajoutée.

- Tests de calcul écrits en rouge puis verts : trajectoires hors contenu, résidus issus de ces trajectoires, budget raster sur trois tailles et fin bornée. Tests de repli pour absence de WebGL, précision insuffisante et compilation échouée.
- Le shader exige `highp` pour éviter un bruit constant sur certaines implémentations `mediump`. Si cette précision manque, l’ouverture est omise. Les GPU et WebView natifs restent à vérifier.
- Références API : [initialisation WebGL](https://developer.mozilla.org/en-US/docs/Web/API/WebGL_API/Tutorial/Adding_2D_content_to_a_WebGL_context), [bonnes pratiques WebGL](https://developer.mozilla.org/en-US/docs/Web/API/WebGL_API/WebGL_best_practices). Le shader est écrit pour ce prototype, sans copie d’un effet tiers.

Validation de cet essai : `pnpm test` (20 tests TypeScript + 19 Rust), `pnpm lint` et build Vite réussis. Revue stricte indépendante : OK avec réserves artistique et native, aucun P1/P2 restant identifié après correction de la précision du shader et arrêt de son calcul hors visibilité. Auto-revue : périmètre limité au prototype, ressources GPU libérées, aucune dépendance ni chaîne visible ajoutée.

Contrôle navigateur macOS : séquences intermédiaires rouge/orange et bleu/violet/cyan observées à 1440 × 900 ; traces fixes après extinction ; interruption par clic et redimensionnement sans canvas résiduel. Animations désactivées : rejouer désactivé et neuf tracés SVG statiques conservés. Fenêtre 390 × 844 : traces recalées et aucun débordement horizontal. Console sans erreur ni avertissement relevé. Le rendu reste soumis au jugement de Louison ; aucune mesure de FPS native n’est revendiquée.

### Ajustement du rythme après retour positif

Ralentissement uniforme à 80 % de la vitesse précédente, y compris du temps du shader : 2,75 secondes au total. Test de non-régression observé en échec puis réussi ; `pnpm test` (39 tests), `pnpm lint` et revue stricte ciblée OK. Séquence ralentie observée dans le navigateur, zéro canvas après sa fin et console sans erreur ni avertissement relevé. La validation native reste ouverte.

## Quatrième essai du 1er octobre 2026 — matière de session conservée

Troisième profondeur locale à la carte de session : reflet passager sous le contenu, zones assombries aux bords haut/droit et bas, lisières irrégulières discontinues, petites fractures et chaleur diffuse. Suppression des trois anciens angles lumineux et de leur observateur de redimensionnement. Le SVG est maintenant un enfant de la carte, sans recalcul de géométrie ni boucle permanente.

Le temps du reflet utilise le même ralenti que les autres couches. Son état temporaire est restauré à la fin, à l’interruption et au démontage. Sans animation ou sans WebGL, seules les marques statiques restent. Aucun asset, dépendance, texte visible ou comportement système ajouté.

Validation de cette tranche : test de chronologie du reflet observé rouge puis vert ; `pnpm test` (21 TypeScript + 19 Rust), `pnpm lint`, build Vite et contrôle du diff réussis. Revue indépendante sans P1/P2 identifié. Reflet et lisibilité inspectés dans les deux thèmes à 1440 × 900 ; mode sans animation et carte à 390 px inspectés, sans débordement horizontal. Après interruption : zéro canvas et style temporaire de surface nettoyé. Console sans erreur ni avertissement. Le jugement artistique et les essais Windows/Tauri restent ouverts.

## Cinquième essai du 1er octobre 2026 — ambiance arrière remplacée

À la demande de Louison, l’identité doit rester perceptible au repos : foyers cuivrés/rouges ou bleus, reflets localisés dans les panneaux, réaction des lisières au survol. Ces éléments CSS restent visibles sans animation.

Une seule couche WebGL de points ajoute jusqu’à 18 braises dans les interstices, derrière les panneaux, après l’ouverture. Les cycles durent 8 à 12 secondes et dérivent lentement. Aucun shader plein écran continu ; dessins plafonnés à 30 Hz, canvas limité à 1,1 million de pixels. Aucune allocation de buffer de particules à chaque image : positions recalculées uniquement après changement de géométrie ou reprise.

L’onglet masqué suspend la boucle et son horloge. L’ouverture suspend aussi cette couche. La désactivation des animations ou la préférence système réduit les effets à leur version statique, sans canvas de braises. Absence/échec de WebGL : repli statique ; perte de contexte : arrêt. Destruction des ressources au démontage et au changement de thème. Ce décor appartient au prototype d’accueil, pas à un overlay en jeu.

Les attributs de diagnostic du canvas mesurent seulement le nombre de dessins et le temps CPU de soumission, sans accès aux données du jeu. Ce n’est pas une mesure de temps GPU ni du coût total de composition. Référence des API vérifiées : [points WebGL](https://developer.mozilla.org/en-US/docs/Web/API/WebGLRenderingContext/drawArrays), [visibilité de page](https://developer.mozilla.org/en-US/docs/Web/API/Page_Visibility_API).

Validation : `pnpm test` (25 TypeScript + 19 Rust), `pnpm lint`, build Vite et contrôle du diff réussis. Tests rouges puis verts pour la distribution bornée et la boucle (plafond, annulation, reprise sans rattrapage), tests de repli WebGL. Revue indépendante : aucun P1/P2 identifié, réserve sur le coût GPU réel.

Navigateur macOS : un seul canvas d’ambiance après ouverture ; zéro en mode réduit et pendant l’ouverture, puis reprise ; bascules de thème sans accumulation ; 390 px sans débordement horizontal ; console sans erreur ni avertissement. Sur un échantillon local de 810 dessins, soumission CPU moyenne 0,021 ms, maximum 0,2 ms (hors GPU, composition et préparation). La pause/reprise de la boucle est testée unitairement ; ouvrir un second onglet via l’outil de contrôle n’a pas rendu le document masqué (`document.hidden` restait faux), donc le déclenchement réel par masquage reste à confirmer en usage natif. Validation artistique et performances Windows/Tauri toujours ouvertes.

## Sixième essai du 1er octobre 2026 — trois profondeurs persistantes

Louison demande des impacts distincts derrière l’app, sur les cartes et devant elles. L’ambiance utilise désormais 12 braises lentes derrière les panneaux, trois foyers de contact locaux (profil, session, progression), et neuf étincelles au premier plan. Les contacts crépitent brièvement sur un bord, puis les étincelles s’en détachent ; les trois foyers sont décalés de trois secondes. Le texte reste au-dessus des contacts, les particules ignorent les interactions. Les palettes suivent le thème rouge/orange ou bleu. Aucun son ajouté.

Les deux canvas WebGL partagent le budget antérieur de 1,1 million de pixels (35 % arrière, 65 % avant), avec une seule horloge plafonnée à 30 Hz. Les sprites sont des points ; les contacts CSS sont localisés et ne couvrent pas tout l’écran. Pas de cumul avec l’ouverture. Mode réduit : zéro canvas et zéro contact animé ; marques statiques conservées. Perte de contexte : nettoyage des deux couches. Aucun asset ni dépendance supplémentaire.

Une sonde GPU facultative utilise [EXT_disjoint_timer_query](https://developer.mozilla.org/en-US/docs/Web/API/EXT_disjoint_timer_query), sans attente bloquante, limitée à six tentatives et 240 dessins. Résultat indisponible ou invalide : arrêt avec état non concluant. Les mesures concernent uniquement les commandes WebGL, hors composition CSS et affichage.

Validation : tests rouges puis verts pour les deux distributions, le budget partagé, la chronologie des contacts et les limites de la sonde ; `pnpm test` (31 TypeScript + 19 Rust), `pnpm lint`, build Vite et contrôle du diff réussis. Revue stricte indépendante OK après correction de la précision partagée du shader et bornage de la sonde ; aucun P1/P2 restant identifié.

Navigateur macOS : deux canvas et trois contacts après ouverture, zéro pendant l’ouverture ou animations désactivées, reprise et changements de thème sans accumulation ; crépitement et détachement inspectés dans les deux thèmes ; fenêtre 390 × 844 sans débordement horizontal. Échantillon sombre local à 1440 × 900 : soumission CPU moyenne 0,056 ms sur 780 dessins (maximum 7,7 ms, préparation comprise), commandes GPU arrière 0,022 ms et avant 0,027 ms en moyenne sur six mesures chacune. Ces échantillons ne prouvent ni le coût total de composition ni les performances Windows/Tauri. La pause/reprise reste couverte unitairement, le masquage natif et le jugement artistique restent à valider.

## Ressources

Les illustrations proviennent de Riot Data Dragon et les polices de Google Fonts. Voir [les sources et licences](../apps/desktop/public/prototype/assets/README.md). Aucune ressource graphique de DPM ou OP.GG n’est utilisée.

## Parcours draft et partage — 1er octobre 2026

Tranche #4 : accueil → arrivée automatique en draft → ban de Lux → pick de Yasuo → composition complète → verrouillage → chargement → vue de l’application en partie. Depuis l’accueil, « Lancer la démo draft » simule la détection après 1,2 seconde. Lecture automatique : étape toutes les 4,2 secondes, chargement de 3 secondes ; pause, reprise, étape suivante et réinitialisation accessibles. Pas de connexion LoL ni de détection système réelle. La vue finale n’est pas un overlay.

La navigation ordinaire utilise désormais une vague de combustion de 800 ms, accompagnée d’un fondu/décalage de 450 ms. L’arrivée en draft et les passages au chargement/en partie utilisent une vague de 1 100 ms ; les équipes/préparation apparaissent avec un léger décalage. Au verrouillage, le panneau des recommandations devient inerte et se replie en 550 ms ; runes et objets s’élargissent. La préférence Animations et le réglage système de réduction des mouvements suppriment ces effets. Aucune animation ne retarde l’accès aux commandes.

Les équipes suivent le côté bleu/rouge, avec le premier pick indiqué à gauche. Les recommandations suivent un ordre scénarisé, excluent le champion banni et changent au pick adverse. L’aperçu ne modifie pas le prépick ; retour conditionnel vers celui-ci. Un prépick banni est effacé. Verrouillage autorisé seulement à l’étape « Composition complète », puis champion conservé jusqu’en partie. Runes partielles et objets sont illustratifs ; noms/icônes Data Dragon, pas de build vérifié ni d’IA réelle. L’import n’effectue aucune action dans League. Les repères de la vue finale, dont l’or, sont fictifs et ne constituent pas une validation de disponibilité/conformité future des données.

Routes partageables `#draft` et `#layouts`, retour navigateur géré ; état de draft conservé pendant la navigation de cette session. Les réglages de simulation du composant draft se réinitialisent lorsqu’on quitte la page. Aucun compte utilisateur ni données privées sur le site. Les autres destinations restent des aperçus du prototype initial.

### Maquette conservée

[Figma — Architecture & wireframes](https://www.figma.com/design/k4SI8lmJungFY3qhYP8ax0?node-id=74-63). Copie visuelle de 19 vues dans `apps/desktop/public/prototype/layouts/`, avec titres d’origine en français et liens individuels dans `manifest.json`. Galerie dans le prototype : « Maquettes de layout ». Ces PNG sont des références statiques, pas une copie éditable ou une synchronisation Figma. L’accès au fichier Figma original dépend de ses permissions ; la copie publiée ne dépend pas de cet accès.

### Vérification de la tranche

Tests rouges puis verts : bans/picks, aperçu/prépick, absence de prépick, côté, verrouillage interdit avant la fin des bans et conservation du champion, routage des liens directs. Contrôles navigateur macOS : parcours Orianna jusqu’en partie, repli du panneau à largeur zéro, navigation par menu et retour navigateur, galerie de 19 vues, thèmes, anglais, mode réduit, 390 px sans débordement horizontal. `pnpm test` : 57 tests au vert ; `pnpm lint`, build Vite et contrôle du diff réussis. Revue stricte indépendante : OK, aucun P1/P2 restant. Scénario automatique jusqu’en partie, côté rouge et console sans erreurs/avertissements vérifiés. L’app native Windows/Tauri et la validation artistique restent ouvertes.

### Publication ChatGPT Sites

Publication réussie le 1er octobre 2026, version 1, accès public par lien demandé par Louison :

- Prototype : https://open-lol-companion-prototype.peon45.chatgpt.site
- Maquettes conservées : https://open-lol-companion-prototype.peon45.chatgpt.site/#layouts
- Draft directe : https://open-lol-companion-prototype.peon45.chatgpt.site/#draft

Identité du site : `appgprj_6abe262593988191920130e220d5cb23`. Déploiement : `appgdep_6abe29b153cc8191b47cb5c2731346a9`, confirmé `succeeded` par Sites. Publication statique depuis une copie dédiée (`Documents/Codex/2026-09-30/sal/open-lol-site`), indépendante du dépôt GitHub. Aucun commit/push du dépôt applicatif. L’archive exclut l’entrée Tauri et les sourcemaps. Manifest d’hébergement dans la copie dédiée ; le réutiliser pour les prochaines publications, sans créer un autre site. Les prochaines versions doivent être republiées explicitement ; aucune synchronisation GitHub/Figma ni automatisation configurée.

## Finition du 1er octobre 2026 — sphères et transitions de feu

Passe limitée au ticket #4. Douze sphères supplémentaires (six derrière, six devant) traversent les panneaux en 12 à 18 secondes, avec un cœur rond, un anneau discret et un halo diffus. Leur lumière varie pendant le déplacement et s’éteint progressivement. Six reflets locaux suivent la trajectoire du premier plan dans les cartes, sous le texte. Les crépitements précédents sont conservés ; aucune dépendance ni chaîne visible ajoutée.

Maximum : 33 points WebGL, deux canvas partageant 1,1 million de pixels et une seule boucle à 30 Hz. Les reflets n’ajoutent aucun contexte GPU. La navigation réutilise le shader de l’ouverture sous forme de vague latérale, sans rejouer le papier opaque ; elle suspend l’ambiance puis libère son contexte. L’interface reste utilisable. Aucun canvas d’ambiance n’est créé sans panneau cible, notamment en chargement/en partie. Mode réduit : aucun canvas ni reflet animé.

Validation : deux tests observés rouges puis verts (distribution des sphères et durée/enveloppe des transitions). `pnpm test` : 59 tests réussis ; `pnpm lint` et build Vite réussis. Revue stricte indépendante : aucun P1/P2/P3 restant identifié. Contrôle navigateur macOS : sphères et vague rouge/bleue inspectées, nettoyage après chargement, mode réduit sans canvas/reflet, largeur 390 px sans débordement, console sans erreur ni avertissement relevé.

Échantillon local sombre à 1440 × 900 : soumission CPU moyenne 0,113 ms sur 630 dessins (maximum 1,9 ms), commandes GPU arrière 0,025 ms et avant 0,050 ms sur six mesures chacune. Ces mesures excluent le coût total de composition CSS/affichage ; elles ne valident pas les performances natives Windows/Tauri. La qualité artistique reste à apprécier en mouvement par Louison et Matthieu.

Publication de la finition : version 2 sur le même lien public, déploiement `appgdep_6abe2f55ea9c8191bced01c6f8338813` confirmé `succeeded`. Les 19 maquettes restent dans la galerie. Le ticket #4 reste ouvert pour validation visuelle ; aucune intégration réelle ni livraison GitHub ajoutée.

## Direction validée et finition draft/build — 1er octobre 2026

Louison confirme son accord et rapporte la validation de Matthieu pour la direction graphique du prototype animé. Ce choix permet de décliner le style sur la draft/build ; il ne valide ni les données fictives ni une intégration réelle. Le ticket #4 reste ouvert pour les déclinaisons restantes et leur revue.

Finition ciblée : équipes compactes avec portrait/poste/nom, côté bleu à gauche et rouge à droite ; aperçu illustré du champion avec portrait du matchup ; rune principale mise en avant et libellés des runes visibles ; chemin d’objets numéroté, noms des objets visibles, alternatives et utilitaires regroupés. Après verrouillage, recommandations masquées et préparation agrandie, avec le plan de jeu et le scaling conservés à côté des runes. Les règles de ban, prépick, retour, verrouillage et import simulé restent identiques.

Socle réutilisé : variables de couleur sombre/clair existantes, Sora pour les titres et Source Sans pour les données, coins asymétriques, accent réservé au choix actif et aux actions, contours bleu/rouge pour les équipes. Aucune nouvelle animation continue ni dépendance. Les données et les builds restent des exemples non validés.

Vérification : interface React/CSS sans nouvelle logique métier (TDD nouveau N/A selon rules/definition-of-done.md). Les tests existants du parcours sont conservés. Contrôles navigateur macOS à 1366 × 768 : boutons de prépick/verrouillage/import visibles, préparation verrouillée entièrement visible, panneau de recommandations à largeur zéro, côtés inversés et retour au prépick fonctionnels. Thème clair et sombre inspectés ; anglais et fenêtre 390 × 844 sans débordement horizontal ni image manquante ; import simulé vérifié. Mode réduit sans canvas ; console sans erreur ni avertissement. Revue indépendante : aucun P1/P2/P3 identifié. Validation native Windows/Tauri toujours à effectuer.

Vérification finale de cette finition : `pnpm test` (59 tests), `pnpm lint`, build Vite et contrôle du diff réussis.

Publication de la draft/build : version 3 sur le même site public, déploiement `appgdep_6abe4333c9a0819197ef2e12321f7460` confirmé `succeeded`.

## Cartes de champions et arbres de runes — 1er octobre 2026

Retour de Louison : runes et objets trop « posés », souhait d’arbres de runes avec les options sélectionnées mises en avant et de cartes de champions immersives. Cette tranche remplace les portraits des équipes par douze illustrations verticales Riot, et les suggestions par des cartes illustrées. Côté, poste, nom, statut personnel et prépick restent visibles ; les cartes prépick sont atténuées avec un contour discontinu. Un prépick banni rend la carte vide, sans conserver son ancien statut.

Les branches principale et secondaire sont affichées avec toutes leurs options éligibles : quatre rangées principales, trois secondaires (sans rune fondamentale secondaire). Les quatre choix principaux et deux secondaires sont éclairés, cerclés et marqués d’une coche. Le nom de chaque rune est accessible au survol et au clavier. Les branches sont consultables et non éditables ; pas de choix parmi les cinq voies ni d’éclats statistiques dans cette tranche. Le scénario utilise Domination/Sorcellerie et conserve l’évolution du matchup et du champion consulté. Sources et licences dans le README des assets, données FR/EN Data Dragon 16.19.1. Aucune recommandation réelle ni connexion au jeu.

Compromis visuel : les cartes et arbres prennent davantage de hauteur, avec défilement normal sur les fenêtres basses. Aucun contenu ou bouton masqué pour imposer un écran unique. Contrôle à 1440 × 900, 1366 × 768 et 390 × 844 ; thèmes clair/sombre, FR/EN, branches de 21/22 options selon la voie, six sélections, retour/prépick/ban/verrouillage et tooltip au clavier vérifiés. Mode réduit sans canvas, aucune image manquante ni débordement horizontal relevé. Les objets conservent leur présentation de la version précédente.

TDD : trois tests de cohérence des fixtures et de traduction écrits en échec puis verts ; un test de rendu protège le retrait du faux statut prépick après ban, observé rouge puis vert. Revue indépendante : P3 de statut corrigé, verdict final OK sans P1/P2/P3 restant. Tests natifs Windows/Tauri et validation artistique toujours ouverts.

Validation finale de cette tranche : `pnpm test` (63 tests), `pnpm lint`, build Vite et contrôle du diff réussis.

Publication : version 4 sur le même site public, déploiement `appgdep_6abe4af6c1cc819189f96f50d4bc0fb8` confirmé `succeeded`.

## Intégration au desktop — 2 octobre 2026 (#4)

Le prototype reste la référence de direction visuelle. L’app Tauri utilise maintenant
ses moteurs `BurnReveal`, `AmbientEmbers`, `PageFlame` et `BurnScars` via
`app/ApplicationEffects.tsx`, sans importer ses données de démonstration ni son layout.
`app/identity.css` porte les surfaces communes des écrans réels : accueil, champions,
joueurs, draft et réglages. Les blocs sans backend restent dans leur état vide.

- Au lancement : combustion centrale, seconde vague dans les interstices et lumière
  sur les panneaux ; les traces persistent dans la carte d’accueil. Un clic ou une
  touche écourte l’ouverture. Le conteneur attend sa ref avant de lancer le moteur.
- Navigation : passage de feu de 800 ms ; arrivée automatique dans une nouvelle phase
  visible de jeu : 1 100 ms. Un rafraîchissement de données ou un changement de phase
  pendant la consultation des réglages ne relance pas l’effet.
- Au repos : deux plans de particules, reflets locaux et crépitements sur trois surfaces
  stables au maximum. Deux résolutions partagent un budget de 1,1 million de pixels,
  avec dessins limités à 30 Hz et pause de la boucle lorsque la page est masquée.
  L’ouverture et les transitions remplacent temporairement les particules ambiantes.
- Le compagnon 3D reste monté dans la barre principale à chaque navigation. Son bouton
  reste contenu dans son emplacement ; seul son canvas décoratif déborde. La marque
  reste un accès séparé à l’accueil. Son interaction et son repli statique sont conservés.
- Animations désactivées ou préférence système de réduction des mouvements : aucun
  canvas d’effet plein écran, compagnon statique, matières et contrastes conservés.
  Sans WebGL, l’interface reste utilisable avec son habillage statique.

La palette ambiante suit le thème (braise sombre / bleu clair). Le personnage conserve
sa couleur propre. Aucune nouvelle donnée de jeu, commande d’import, API ou fonction
backend n’est ajoutée. Le passage réel de phase avec League et le rendu Windows restent
à valider lors des essais dédiés ; les tests simulés ne remplacent pas cette validation.


### Finition desktop : défilement et bibliothèque compacte (2 octobre 2026)

- Le bouton Retour est une flèche dans la barre supérieure, avec nom accessible FR/EN et emplacement stable (désactivé si aucun historique). Il conserve les filtres, la sélection et la position des listes ; aucune ligne Retour dans le contenu.
- La bibliothèque Champions commence directement par recherche, filtres, tri et cartes. Le titre reste dans la navigation/le nom accessible de la zone ; introduction et slogan sont retirés. Les filtres tiennent sur une ligne à partir de 680 px de largeur **du panneau**, sinon sur deux lignes. La fiche du champion reste à droite.
- `overscroll-behavior: none` sur le document et les zones défilantes du shell désactive le rebond et la propagation au bord des zones. À la taille desktop (>=760 px), html/body/root sont bornés et non défilants. Les listes conservent `overflow:auto`, inertie et défilement clavier ; aucun gestionnaire `wheel` bloquant, aucune préférence macOS modifiée. Support WebKit documenté dans les [notes Safari 16.4](https://developer.apple.com/documentation/safari-release-notes/safari-16_4-release-notes).

### Finition de la bibliothèque (2 octobre 2026 — #4, #61)

Les portraits conservent le ratio 308/560 sans zoom au survol. Le cadre arrondi contient les filtres et le compteur, avec une marge autour des cartes. Les menus Classe/Tri et les quatre filtres de builds utilisent le même composant : flèches, Début/Fin, Entrée, Échap, Tab et recherche par lettres ; le menu sort du cadre défilant sans être coupé.

Les compétences montrent d'abord les délais, coûts de ressource et portées vérifiés du catalogue, avec chaque rang et les constantes compactées. Le fonctionnement détaillé reste repliable ; les termes dégâts, boucliers et soins sont colorés et les nombres mis en avant. Les mêmes valeurs figurent dans les infobulles et détails. Le coût numérique n'est pas présenté comme un coût total : un supplément encore non résolu est signalé (ex. PV du Z de Soraka). Les portées sont les unités de la source, sans transformer une valeur technique en conseil.

Limite du catalogue #61 : les formules de dégâts, boucliers et soins de nombreux sorts ne sont pas résolues ; aucune valeur ni ratio AP/AD n'est inventé. Leur affichage numérique complet nécessite cet enrichissement en amont. Le passif reste descriptif lorsqu'aucune statistique vérifiée n'existe.

### Densité du panneau Compétences (2 octobre 2026 — #4)

L'onglet Compétences utilise un en-tête de 72 px et regroupe touche, nom et statistiques de chaque sort en lignes compactes. Les statistiques générales disponibles et reconnues passent dans un composant Disclosure partagé et repliable, fermé initialement (11 pour Ahri, sans AP fictif ajouté). Les descriptions restent dépliables individuellement ; les valeurs par rang, notes de coût et accès aux détails des sorts sont conservés. Le texte ne diminue pas pour forcer l'affichage ; les métriques reviennent à la ligne et seul le corps de fiche défile quand nécessaire. Builds et Catalogue conservent leur présentation.

Recette représentative actualisée : à 1280×720, Ahri affiche son passif et les paramètres de ses quatre sorts ; le bas de fiche reste accessible par défilement interne. À 960×600, les compétences, ouvertures de descriptions/statistiques et variations de ressource non chiffrées gardent un défilement interne, sans débordement global. Ce résultat ne garantit pas l'absence de défilement pour tous les noms, langues ou descriptions ouverts.

### Icônes et couleurs des statistiques

La famille des pictogrammes des infobulles League 16.19 est embarquée localement, avec rectangles et provenance dans `statIcons.json` et `public/game-data/stats/README.md`. AD orange, AP violet, armure or, RM cyan, PV vert, mana bleu ; les valeurs restent accompagnées des noms. Les glyphes clairs reposent sur un petit fond sombre dans le thème clair pour rester lisibles. Aucun appel réseau n'est nécessaire au rendu. Le formatage conserve trois décimales pour la vitesse d'attaque et reconnaît les unités de portée.

Une expression comme `60 % AP`, uniquement lorsqu'elle est déjà écrite dans une description affichable, reçoit la couleur AP et son icône sans changer le texte ni calculer le coefficient. Les dommages magiques ne deviennent pas de l'AP. Les formules encore non résolues du catalogue #61 restent absentes : ce lot ne fournit pas de scalings manquants. Les statistiques inconnues conservent un affichage textuel, sans pictogramme arbitraire.

Les paramètres d’un sort utilisent trois colonnes stables : pictogramme et libellé au-dessus, valeurs alignées sous le libellé. Les glyphes et leur atlas sont redimensionnés ensemble (14 px dans les paramètres et les mentions, 16 px par défaut). Le fonctionnement du sort reste dépliable. Le coût reçoit sa couleur uniquement si la ressource est explicitement rattachée à `{{ cost }}` ou si la référence générique peut être résolue avec la fiche du même champion : mana bleu, énergie jaune, PV vert. L’énergie n’a pas de pictogramme dans l’atlas actuel ; aucun substitut n’est inventé. Un coût inconnu reste neutre.

Les mentions `/s` et `/ roquette` sont conservées lorsqu’elles qualifient directement le coût. Les formules restantes restent signalées par « Autre variation de ressource non chiffrée » : cela couvre aussi bien un supplément de PV (Soraka) qu’une restauration d’énergie (Akali), sans les transformer en valeurs connues. Le même composant sert à la fiche, aux infobulles et aux détails.


### Contrôles partagés et transitions de panneaux — 2 octobre 2026

`ui/SelectField` est le sélecteur unique : `label`, `value` et `onChange` contrôlés, `options` (`value`, `label`, `disabled` facultatif), avec `disabled`, `id`, `className` et `placeholder` optionnels. Les valeurs demeurent des chaînes ; les conversions métiers restent chez l’appelant. Il sert aux filtres Champions, régions/joueurs, réglages/imports, préparation, runes/sorts, variantes de builds et au prototype. Une garde de test refuse les sélecteurs HTML natifs dans les sources runtime. Les dialogues système restent gérés par l’OS.

Le focus reste sur le bouton : flèches, Home/End, recherche par frappe, Entrée/Espace, Échap et Tab. Les options interdites ne peuvent être choisies à la souris ou au clavier. Valeur inconnue : placeholder, sans afficher arbitrairement la première option. Portail dans le dialogue actif pour respecter la modalité ; fermeture au clic extérieur, changement de taille ou défilement extérieur.

La fiche champion distingue identité, onglets, statistiques générales repliables et cartes de compétences. Icône, touche et nom identifient chaque sort ; métriques colorées toujours visibles, explication secondaire dépliable avec `Disclosure`. La hauteur se développe et se replie, les parties cachées sont retirées de la navigation clavier.

`usePresence` conserve les panneaux pendant leur sortie (180 ms), annulable à la réouverture. `useDialogMotion` garde les dialogues modaux jusqu’à la fin du fondu, neutralise les interactions durant la sortie et conserve la restitution du focus. Les fenêtres modales utilisent un fondu sans transform pour ne pas déplacer le repère des dropdowns portalisés. Fiche champion et menu latéral glissent légèrement ; compte et recherche apparaissent avec un mouvement bref. Aucun nouvel effet WebGL ni animation en boucle. Préférence de l’app et `prefers-reduced-motion` neutralisent ces mouvements.

Le lien « Détails du champion » en bas de l’onglet Compétences est retiré : il doublonnait la fiche déjà ouverte. Les statistiques de base restent disponibles en haut du panneau et chaque icône de sort ouvre toujours ses détails.
