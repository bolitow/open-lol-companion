# Recette du front — passe nocturne du 1er octobre 2026

Tickets : #8, #12, #13, #14, #15, #16. Travail local sur `main` à partir de `6d684324d17e82c7d095afecf21ca7b30be5a20f` avec changements préexistants conservés. Aucun commit, push, fusion ou déploiement. Plan avant code : [plan-front-nuit](../plan-front-nuit.md).

## Code et contrôles automatisés

- Import explicite de la variante d’objets visible, composants et répétitions conservés. Moteur `items.rs` identique à la PR #59, révision `cf04f75b21e79049cdb044192c0384cf0bde1460` ; ni catégories artificiellement combinées, ni import automatique.
- Personnalisées Faille acceptées seulement avec contexte réel `isCustomGame`, carte 11 et mode CLASSIC. Équipes incomplètes et bots pris en charge ; autres cartes/modes refusés. Gameflow tardif et sortie de sélection couverts.
- Consultation et filtres conservés entre pages ; retour au champion local dans une nouvelle draft. Réponses d’import périmées ignorées. Un cycle de draft entièrement survenu pendant une coupure ne peut être identifié sans identifiant de session : limite conservée explicitement.
- Cartes consultables et accessibles, retour nommé vers le champion local, passif chargé à la demande et trois statistiques maximum par infobulle. Les brouillons non importés des éditeurs restent éphémères lorsqu’on quitte la page.
- TDD rouge puis vert des comportements ajoutés ; revue indépendante finale **OK**, y compris correction des libellés accessibles FR/EN et du fond des options natives.
- `pnpm test` : **489 tests réussis** (130 desktop, 10 shared, 349 Rust), deux recettes réelles optionnelles ignorées. PostgreSQL local utilisé pour les tests qui en ont besoin.
- `pnpm lint` : typage TypeScript, format Rust et Clippy sans erreur. `git diff --check` sans erreur.

## Interface avec données synthétiques

Vrais composants React et catalogues locaux, bridge de recette explicitement synthétique : ce contrôle ne prouve pas un échange avec League.

- 960×600 et 1280×800 : aucun débordement global du document ; défilement local des contenus longs.
- Sélection de Jinx depuis sa carte, puis retour « Votre pick · Ahri » ; Ahri/Support conservés au retour des réglages.
- Infobulle de passif au clavier, français/anglais, thèmes sombre/clair.
- Mouvement réduit : animation des cartes à zéro ; préférence initiale restaurée ensuite.
- Variante d’objets simulée acceptée avec un message d’acceptation, sans prétendre confirmer l’ordre réel de la boutique.

Capture : `sal/outputs/draft-nuit-960x600.png`, bandeau « données synthétiques » visible. HTML temporaire retiré du dépôt ; viewport réinitialisé et onglet de recette fermé.

## macOS natif

Bundle Tauri debug construit et ouvert. Cache Cargo isolé utilisé : le cache initial contenait des artefacts iCloud `dataless` qui bloquaient le compilateur ; aucun fichier préexistant supprimé. Le bundle final inclut le correctif de contraste Windows.

Essai League réel **partiel** : première ouverture en état ReadyCheck incohérent (file -1), puis relance et accès au formulaire de partie personnalisée masquée sans spectateur. La confirmation n’a pas permis d’atteindre un salon ; la lecture gameflow n’a pas fourni de session exploitable à ce stade. Cette absence seule ne démontre pas un défaut du connecteur. Aucun import, changement de sorts ou de rune n’a été réalisé sur ce client. Client fermé avant le passage Windows. Le verrouillage ultérieur du Mac a interrompu la poursuite visuelle.

Il reste donc à vérifier sur macOS une vraie entrée Lobby → ChampSelect, les imports et leurs confirmations, puis sortie/reconnexion. Aucun résultat synthétique n’est présenté comme validation de ce parcours.

## Windows natif

Snapshot transmis au chat Test Windows : 3 851 fichiers vérifiés par manifeste, aucune différence à la réception. SHA256 du ZIP initial : `55fa6ca56bd04f3f3044a0ff8377402203eb15120dfaa745103a0152ca64de87`.

Compilation Tauri, 140 tests TypeScript et 95 tests Rust ciblés réussis sur ce snapshot. Aucun test PostgreSQL exécuté sous Windows pour cette passe ; pas de revendication de lint Windows complet. Navigation native vérifiée : retour Accueil → Retour conservant Aatrox, Top et l’onglet Catalogue ; parcours traduit en anglais contrôlé.

La recette a révélé un menu de champions aux libellés trop clairs sur fond blanc, alors que le menu Poste restait lisible. Correctif unique transmis après le snapshot : couleurs explicites `select option` et `select optgroup` dans `apps/desktop/src/ui/theme.css`. SHA256 final identique sur Mac et Windows : `cb0dc826c1fcecd8837daf10ba5d356e0d78c5f76e8a7611530e4724e77abd15`.

État au 1er octobre : reconstruction réussie, mais relancement interrompu par une erreur WebView2 avant affichage. Le chat Test Windows attendait une autorisation locale pour poursuivre le diagnostic, qui a ensuite expiré. La cause n’était pas établie. **À cette date, correctif de contraste pas encore confirmé visuellement et imports réels du nouveau snapshot non validés.** Le harnais de sauvegarde/restauration et le dossier de recette ont été conservés sur la tour ; aucune ancienne recette n’a été réutilisée comme preuve. Voir la reprise ci-dessous pour l’état actualisé.

## Livrables et limites

### Reprise du 2 octobre 2026

Sources revérifiées contre les 3 851 empreintes du snapshot transmis : seule différence dans les sources, le CSS d’options déjà recompilé et testé. Empreinte du bundle Mac inchangée. Aucun nouveau comportement modifié pendant cette reprise.

- **Mac** : le bundle démarre et observe `Client connecté · Hors partie` avec League réel. League échoue ensuite à connecter le compte (« Une erreur inattendue est survenue pendant la connexion »). Après fermeture et redémarrage du lanceur, crash Riot : `SIGKILL (Code Signature Invalid)`, terminaison `CODESIGNING / Taskgated Invalid Signature`. `codesign --verify --deep --strict` signale un sous-composant Riot non signé et une signature/Info.plist incohérente sur l’app League. Même le lancement depuis Finder ne démarre plus le client. Aucune modification de signature ou de l’installation n’a été tentée ; aucun import réel effectué. La réparation de l’installation officielle Riot/League reste un prérequis à la fin de recette Mac.
- **Windows** : même binaire et profil relancés dans la session utilisateur normale, préférence English conservée. Erreur WebView2 non reproduite dans ce contexte ; cause initiale non établie. Menus Champion et Poste ouverts en sombre/clair, option sélectionnée comprise : lisibles. Correctif de contraste visuellement confirmé.
- **Objets, moteur réel Windows** : import et réimport réussis, puis restauration des quatre sets initiaux. Ce contrôle du moteur ne valide ni le bouton UI avec statistiques de production, ni la priorité visuelle dans la boutique.
- **Sorts, UI réelle Windows** : import depuis l’app réussi dans la sélection personnalisée, Flash sur F et relecture concordante, puis paire initiale restaurée. Aucun import de runes effectué.
- **Incident de recette** : l’entrée personnalisée était masquée et sans spectateurs. La commande de sortie Windows a échoué (`SendInput sent 0 of 1 events; GetLastError=87`), puis le compte à rebours a lancé le processus de jeu malgré l’absence de verrouillage manuel. Ce lancement n’était pas prévu ni accepté comme scénario de recette. Le processus a été arrêté ; le client est resté en Reconnect et la sauvegarde du harnais a été conservée en mémoire. Aucun nouveau test chronométré n’a été lancé. La session est ensuite revenue spontanément hors partie, sans relancer le jeu ni appeler la commande early-exit envisagée. La recette des runes reste non validée.
- **Restauration finale Windows** : le harnais a terminé avec succès après le retour hors partie. Contenu et identifiants des dix pages de runes, page active initiale, ordre et contenu des quatre sets identiques à la sauvegarde. Paire de sorts restaurée avant le lancement involontaire ; skin inchangé lors de la relecture. Il faut un mécanisme de sortie fiable, indépendant du clic UI et vérifié avant toute nouvelle recette chronométrée.

League, le processus de jeu et le harnais Windows sont fermés. Environnement et preuves conservés dans `C:\Users\Peon4\Documents\Codex\2026-09-30\test\outputs\night-windows-qa\compte-rendu.txt` et le dossier de travail associé. Le bundle Mac est laissé ouvert pour consultation ; Riot/League Mac ne tourne plus. Les 489 tests et le lint Mac sont les validations de la passe initiale sur ces mêmes sources, pas une nouvelle exécution pendant cette reprise documentaire.

Le chat Test Windows a signalé une URL interne sensible produite par la lecture d’accessibilité du client League dans une sortie d’outil. Cette méthode a été abandonnée au profit des captures ; cette URL n’a été recopiée dans aucun fichier de recette, ticket ni livrable. Les sauvegardes de réglages sont restées en mémoire.

Diagnostic Mac ciblé conservé dans `sal/work/night-front/reprise-2026-10-02/mac-diagnostic.json` ; aucun secret ni donnée de compte dans cet extrait.

Bundle Mac : `sal/outputs/Open LoL Companion.app`, exécutable SHA256 `eaef185a92b1ccbd0f48fa3bb2163d0d64d8e7473ef9485a415ef876b250d56a`. Journaux et copie de référence avant changements : `sal/work/night-front/`. Les tests réels publics/classés et la priorité réelle des objets dans la boutique sont différés par choix utilisateur. Aucun lancement de partie pendant la passe du 1er octobre ; lancement privé involontaire lors de la reprise du 2 octobre, décrit ci-dessus. ARAM, entraînement, coopératif et modes temporaires restent hors périmètre de cette passe. L’API de statistiques de production doit encore être configurée ; aucune donnée de recette n’est embarquée dans le bundle livré.

Tickets #8, #12, #13, #14, #15 et #16 mis à jour et relus après publication ; #13–16 actualisés le 2 octobre. [Bilan GitHub commun](https://github.com/bolitow/open-lol-companion/issues/13#issuecomment-5940970073). Ils restent ouverts. Le serveur temporaire de transfert est arrêté. Au 1er octobre, le client League Mac et l’app de recette avaient été fermés ; au 2 octobre, seul le bundle du compagnon Mac reste ouvert. Le cache Cargo temporaire de 6,6 Go a été supprimé après copie du bundle final ; journaux et sources de recette utiles sont conservés. Vite préexistant sur 1421 est conservé. Le lot 5 reste partiel jusqu’aux vérifications natives manquantes.
