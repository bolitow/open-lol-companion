# Amis du client LoL — #71

L'accueil affiche les amis du compte actuellement connecté : nom, présence normalisée, compteur et accès au profil lorsque Riot ID et plateforme sont complets. Les avatars utilisent des initiales. Aucune région n'est déduite du tag ou du compte actif ; les identités incomplètes restent visibles sans lien de profil. Le suivi manuel de joueurs reste ultérieur.

Lectures Rust uniquement : `/lol-chat/v1/session` et `/lol-chat/v1/friends`, contrat vérifié dans le [Swagger extrait du client](https://raw.githubusercontent.com/KebsCS/lcu-and-riotclient-api/main/lcu/swagger.json). `gameName` + `gameTag` forment le Riot ID. Les valeurs `chat`, `away`, `dnd`, `offline`, observées sur le client réel, deviennent connecté, absent, occupé, hors ligne ; toute autre valeur reste inconnue. Aucun état de partie, champion ou chronomètre n'est déduit de la présence.

Une session sociale `loaded` est nécessaire pour annoncer une liste prête, y compris vide. Le compte est relu avant/après, la liste est effacée immédiatement à la déconnexion/changement de compte, les réponses anciennes sont rejetées par génération. Un producteur dédié relit toutes les trente secondes et annule sa lecture sur changement de compte. Les mises à jour sociales ne réveillent pas le producteur Live Client.

Commande `friends_state` et événement `friends-state` réservés à la fenêtre principale. DTO partagés Rust/TypeScript ; pas de mot de passe, PUUID, identifiant de conversation, message, note ou persistance de liste. Aucun appel à l'API Riot publique, aucun changement du quota de collecte. L'abonnement frontend précède la lecture et reprend deux fois à une seconde d'intervalle après un échec initial.

Recette GET seule macOS du 2 octobre 2026 : session prête, 98 entrées affichables, dont trois identités avec une plateforme utilisable pour ouvrir un profil public. Tests projection, états vides/chargement/erreur, changement de compte, réponses tardives, FR/EN et cent amis. Validation du client réel Windows encore nécessaire ; le transport local réutilise le connecteur multiplateforme existant.

Recette finale dans l'app macOS compilée : accueil affiché à 1280×800, 98 amis dont cinq connectés, liste avec défilement interne, compte actif et dix parties locales visibles simultanément. Vérification native de l'accessibilité et du rendu clair ; aucune réponse nominative sauvegardée dans le dépôt.
