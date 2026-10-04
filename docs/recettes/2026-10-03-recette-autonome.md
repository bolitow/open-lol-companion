# Recette native autonome — 3 octobre 2026

Tickets : #8, #11, #22, #23, #63, #64, #65 et #71.

## Plan et limites de preuve

Inventorier les critères ouverts, identifier les binaires, tester les parcours natifs sur macOS et Windows, restaurer les préférences puis publier les résultats dans les tickets existants. Les tests automatisés, une ancienne recette et un aperçu hors partie ne remplacent pas une recette en jeu de la version concernée. Aucune clôture sur la seule base d'une compilation.

Le code produit n'est pas modifié dans cette passe. Les changements Collection et interface déjà présents dans le checkout principal sont conservés et ne font pas partie de la validation de `main`.

## Versions

- Référence commune publiée : `600ba7fb97391e05f5608965a4b33413b342abf4` (PR #76 incluse).
- macOS 27.0, build 26A428, Apple Silicon ; application Tauri native de développement construite dans le worktree isolé `desktop-reserves`.
- SHA-256 du binaire Mac de cette référence : `1f6d78dff1826a2deab4645653956d85125dc819922f5d25a6e803eacc5ade76`.
- Les premiers essais diagnostics/autostart/fermeture Mac ont été faits sur le binaire de travail `9bb7a43` avec modifications locales, SHA-256 `8f166bd0ac2b0b78a5e6632546d7e5727dffec324e8312a2349e0862cfbb7900`. Les modules natifs concernés ont été comparés à `600ba7f`, mais cela ne transforme pas ce binaire en build de `main`.
- Windows 11 Pro 25H2, build 26200.9457 : résultats délégués au chat « Test Windows », distincts des recettes des 1er et 2 octobre. SHA-256 du binaire testé : `7d5ae08a5864bd57dd323716430acc9490695bb93b74a378b4cd3bb3e8a02804`.

## macOS — résultats observés

### Réglages et diagnostics (#11)

- Activation du démarrage automatique : entrée LaunchAgent créée avec `RunAtLoad=true` et argument `--autostart`. Annulation : option OFF et fichier retiré. Une nouvelle ouverture de session macOS n'a pas été exécutée.
- Export natif sans fichier League : sauvegarde réussie, ZIP valide contenant exactement `manifest.json`, `app-events.json`, `league-summary.json` ; résumé League vide.
- Export avec un fichier synthétique de quatre lignes : résultat de quatre lignes, un marqueur ERROR, un WARN et un INFO. Aucune ligne brute du fichier ne figure dans l'archive.
- Fermeture en arrière-plan OFF : la fermeture de fenêtre arrête le processus ; plusieurs relancements réussis, dont pendant la draft et pendant la partie.
- Valeurs finales restaurées : fermeture en arrière-plan ON, autostart OFF, français, sombre, animations ON, Flash D ; runes/objets automatiques OFF, minimum 100, poste personnalisé non choisi ; overlay OFF, exclusif OFF, écran 1, gauche 2 %, haut 18 %, largeur 20 %, opacité 90 %.
- Menu tray complet FR/EN, ouverture d'une nouvelle session OS et accélération matérielle : non validés par cette passe.

### Compte, amis et profils (#64, #65, #71)

Sur `600ba7f`, démarrage avec League déjà connecté : profil, niveaux/rangs et dix parties réelles affichés. Les amis et leur présence sont reçus ; une identité complète permet d'ouvrir la page Joueurs avec le Riot ID et la région correspondants. Une identité incomplète reste non cliquable.

Fermeture normale de League via « Quitter », sans déconnexion du compte Riot : la page Joueurs consultée est conservée ; l'accueil affiche « League déconnecté · compte conservé », le profil et l'historique restent visibles, les amis affichent une invitation à rouvrir League. Relancement par le bouton Jouer de Riot Client : reconnexion automatique et retour des amis, sans saisir d'identifiants.

Le service public de profils n'est pas configuré dans cette installation : l'erreur explicite est affichée, aucun profil distant complet n'est validé. Après reconnexion, plusieurs identités sociales restent incomplètes et sont présentées comme indisponibles. Aucun changement A → B dans une même instance ni changement de région n'a été réalisé ; deux machines avec des comptes différents ne prouvent pas ces scénarios.

### Partie privée, navigation et Live (#8, #23, #63)

Partie personnalisée en mode aveugle, salon masqué, groupe fermé, spectateurs désactivés, un bot adverse et aucun autre joueur humain. Aucun matchmaking public.

- L'accueil bascule automatiquement vers la draft ; prépick Ahri reçu et affiché.
- Fermeture complète puis relancement de Companion pendant la draft : retour à la draft avec le prépick et le temps restant.
- Ouverture de Réglages avant le lancement du jeu : cette page reste ouverte quand l'état passe à « En partie ». « Revenir à la session » mène au Live.
- Champion Ahri, niveau 1, K/D/A 0/0/0 et CS 0 cohérents avec le HUD observé. Le chrono progresse (0:10, 0:41 puis 0:53 après relancement) ; les captures ne sont pas simultanées et ne prouvent pas une latence chiffrée.
- Relancement complet de Companion en partie : retour direct au Live, sans données de l'ancienne draft affichées comme une autre partie.
- Statistiques de recommandations explicitement indisponibles : service non configuré et contexte personnalisé incomplet. Aucun import réel de runes, objets ou sorts validé dans cette passe. L'option d'auto-import des sorts ajoutée sur `main` doit être testée séparément en OFF et ON ; l'ancien critère « sorts toujours inchangés » ne suffit plus.
- Sortie de cette partie privée et retour de League à l'accueil vérifiés. Réglages reste ouvert, état « Hors partie ». Après réouverture du binaire de travail habituel, l'historique contient la partie Ahri de 4:47 ; cette dernière observation appartient au binaire de travail, pas à `main`.

Lors de cette première passe, le parcours de fin naturelle avec bilan, une deuxième partie et la variation des K/D/A/CS ne sont pas validés. L'outil de contrôle a eu des actions souris sans effet observable dans le jeu ; la sortie a été obtenue par le menu normal du jeu et sa confirmation. Un message de lancement invalide est apparu en réinterrogeant l'ancien handle de jeu après sa fermeture ; il a été acquitté. Ce message n'est pas attribué à Companion.

### Overlay (#22)

Activation et sauvegarde de la préférence en partie, puis désactivation et restauration vérifiées. Les captures de la fenêtre du jeu ne prouvent pas la composition de la fenêtre superposée, et les clics observés ne suffisent pas à valider le passage à travers le panneau. Aucun verdict positif sur le focus, les clics, le plein écran, le multi-écran ou le budget FPS/CPU pour ce binaire dans cette passe. Les recettes antérieures de `main` restent des preuves distinctes.

## Windows — résultats rapportés par le chat de test

Rapport local : `C:/Users/Peon4/Documents/Codex/2026-09-30/test/outputs/windows-main-2026-10-03/recette-finale.md`, avec `restoration-verification.json` et les preuves référencées. Les résultats suivants proviennent de ce chat ; ils n'ont pas été observés directement depuis le Mac.

- Diagnostics : annulation du dialogue de sauvegarde correctement signalée ; exports sans log et avec fichier synthétique réussis, exactement les trois entrées prévues. Résumé du fichier synthétique : cinq lignes, un marqueur ERROR, deux WARN et deux INFO, sans texte brut, nom ou chemin du fichier.
- Refus des fichiers binaires et des fichiers de plus de 8 Mio observés.
- Démarrage automatique : activation crée l'entrée `Run`, annulation la supprime. **Réserve :** la valeur `StartupApproved\\Run\\Open LoL Companion`, absente au début, reste après annulation. Ce résidu a été retiré uniquement pour remettre la machine en état ; le produit n'est pas corrigé par cette recette. Pas de nouvelle ouverture de session Windows testée.
- Recherche « moins d'animations » conservée après navigation ; annulation de Flash sans annuler langue/thème ; persistance après fermeture réelle observée.
- Aperçu overlay hors partie, passage des clics vers Companion, arrêt manuel et raccourci Ctrl+Alt+O observés. Cela ne valide pas l'overlay au-dessus d'une vraie partie.
- League ouvert au départ mais identité `unnamed=true` et session sociale non prête ; Companion refuse l'identité incomplète. Après relancement, Riot demande une connexion manuelle : les scénarios compte/amis connectés, draft, imports et partie réelle restent bloqués pour cette passe. La déconnexion LCU conserve l'écran Réglages.
- Menu de la zone de notification non accessible par l'outil de test : commandes tray non validées.
- 296 tests TypeScript, 197 tests Rust ciblés, typecheck et compilation Tauri réussis. Le `pnpm test`/`pnpm lint` complet Windows n'est pas revendiqué ; collecte Riot et PostgreSQL non réexécutés.
- Nettoyage rapporté comme vérifié : processus de test arrêtés, profil WebView2 temporaire, nouveaux réglages, clone et build temporaires retirés ; entrées de démarrage absentes et travaux préexistants conservés. Les preuves et le rapport sont conservés.

## Vérification automatisée de la référence commune sur Mac

Exécutée avec Node 24.19.0 et pnpm 10.28.0 via Corepack :

- `pnpm test` : succès, 296 tests TypeScript et 459 tests Rust ; deux tests Rust existants ignorés.
- `pnpm lint` : succès (TypeScript, format Rust et Clippy).
- `pnpm --filter @olc/desktop tauri build --debug --bundles app` : succès.

Ces résultats concernent `600ba7f`, pas le diff Collection/interface non commité.

## Rangement et reprise

Preuves locales non versionnées : `work/recette-autonome-2026-10-03/` (journaux de build/tests/lint, deux ZIP de diagnostic synthétique, fixture et capture des réglages restaurés). Aucune donnée de compte ni archive brute publiée dans GitHub.

Deux références Git invalides dont le nom finissait par « 2 » empêchaient `fetch` ; elles ont été déplacées, sans perte, dans `git-ref-backup/` sous ce dossier de recette. Les branches réelles et les fichiers de travail n'ont pas été réinitialisés. Les autres fichiers préexistants portant ce suffixe sont conservés.

À l’issue de la première passe, le binaire de recette isolé était arrêté, la version native de travail habituelle rouverte à l’accueil, League connecté à l’accueil et les préférences restaurées. Le bundle isolé et les preuves sont conservés pour reproduction. Aucun commit, push, fusion ou déploiement web dans cette passe.

## Décision de clôture

Aucun des huit tickets n'est entièrement validé par cette passe. Les résultats partiels doivent être reportés sans cocher une validation OS globale : #8 attend notamment la fin naturelle et Windows ; #11 le menu tray complet et une ouverture de session ; #22 la matrice en jeu/performance ; #23 la couverture restante Windows et les variations réelles ; #63 les imports sur données configurées ; #64 le service public ; #65/#71 le changement de compte dans la même instance.

## Traçabilité GitHub et revue

Commentaires publiés et relus après écriture : [#8](https://github.com/bolitow/open-lol-companion/issues/8#issuecomment-5968286442), [#11](https://github.com/bolitow/open-lol-companion/issues/11#issuecomment-5968286578), [#22](https://github.com/bolitow/open-lol-companion/issues/22#issuecomment-5968286770), [#23](https://github.com/bolitow/open-lol-companion/issues/23#issuecomment-5968286962), [#63](https://github.com/bolitow/open-lol-companion/issues/63#issuecomment-5968287175), [#64](https://github.com/bolitow/open-lol-companion/issues/64#issuecomment-5968287448), [#65](https://github.com/bolitow/open-lol-companion/issues/65#issuecomment-5968287683), [#71](https://github.com/bolitow/open-lol-companion/issues/71#issuecomment-5968287963). Aucun ticket fermé ni case OS globale cochée.

Auto-revue et revue stricte indépendante : **OK** pour ce rapport et son entrée CHANGELOG. Les réserves de recette restent celles décrites ci-dessus ; ce verdict documentaire ne valide pas les fonctionnalités incomplètes.

## Reprise — entraînement solo et blocages confirmés

Même binaire Mac `600ba7f`, sans recompilation ni modification produit. Le worktree isolé contient une modification de configuration pnpm (`allowBuilds`), laissée intacte : aucune nouvelle validation de cette configuration n'est revendiquée.

- Le client League, d'abord noir en capture, s'affiche normalement après remise au premier plan ; aucun redémarrage ni déconnexion nécessaires.
- Lancement d'une deuxième session, cette fois dans l'outil d'entraînement, groupe fermé, un seul humain et aucun bot. Depuis l'accueil, Companion passe automatiquement à Draft puis à Partie.
- La draft d'entraînement signale explicitement « Ce mode de sélection n'est pas encore pris en charge » et ne reçoit pas le prépick Annie. Cela limite la couverture des modes de #63 ; la navigation de #8 reste observable.
- Une fois le jeu chargé, Live affiche Annie niveau 1, K/D/A 0/0/0, CS 0 et un chrono neuf (0:16 puis progression). Les anciennes données Ahri ne restent pas affichées. Preuve locale : `work/recette-autonome-2026-10-03/reprise-entrainement-live.txt`.
- Les clics et le défilement envoyés au jeu restent sans effet observable, y compris après remise au premier plan ; les touches ouvrent les menus et le chat. La commande officielle `/surrender` répond qu'il est trop tôt. Ni niveau augmenté, ni victoire d'entraînement, ni bilan de fin naturelle ne sont validés.
- Le menu natif Quitter ouvre la confirmation, mais les tentatives de confirmation ne terminent pas la session. Une intervention manuelle limitée au bouton Quitter a été demandée. La restauration attendait alors ce clic ; sa confirmation est consignée ci-dessous.
- À 12 h 39 (Paris), le chat Test Windows confirme que Riot affiche toujours son formulaire de connexion, champs vides, sans fenêtre League. Aucun nouveau build ni changement de réglages exécuté. Les scénarios Windows #8/#65/#71 restent bloqués par la connexion manuelle.

La deuxième session est donc couverte partiellement sur Mac ; la fin avec bilan et les scénarios de changement de compte restent ouverts. Aucun ticket ne peut être fermé sur la base de cette reprise.

Restauration partielle de la reprise : binaire de recette quitté par son menu natif ; application de travail habituelle rouverte et affichant la session Annie encore active. Aucun réglage Companion modifié pendant cette reprise. La fermeture de la session solo attendait alors le clic humain demandé. Revue stricte documentaire : OK après clarification de l’état historique de la première passe.

Compléments GitHub publiés puis relus : [#8](https://github.com/bolitow/open-lol-companion/issues/8#issuecomment-5968462371), [#23](https://github.com/bolitow/open-lol-companion/issues/23#issuecomment-5968462505), [#63](https://github.com/bolitow/open-lol-companion/issues/63#issuecomment-5968462660).

### Confirmation après intervention utilisateur

Louison confirme avoir quitté. Vérification directe : processus de jeu absent, client League à l’accueil connecté, application native habituelle à l’accueil avec compte et amis affichés. Le binaire isolé de recette n’est plus actif. La restauration de cette reprise est terminée.

Cette sortie volontaire ne valide pas une fin naturelle avec bilan. Le retour à l’accueil est observé sur le binaire de travail `9bb7a43` avec modifications locales, et ne doit pas être attribué à `600ba7f`. Aucun nouveau résultat Windows.
