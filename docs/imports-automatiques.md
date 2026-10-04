# Imports au prépick — #63, partie client

Plan appliqué : réutiliser les imports #14/#15/#16 et le transport de builds Rust,
projeter l'identité locale de draft et la vraie file, sélectionner les variantes par
champion/poste, puis contrôler la draft juste avant chaque écriture. Tests de
concurrence et des gardes LCU avant raccordement du panneau. Ce lot repose sur
le front de préparation #13 et concerne la section 5.4 du cahier des charges.
Les overlays #22/#23 et le travail #27 restent hors périmètre.
Après essai utilisateur : déclencher dès le prépick, accepter une personnalisée
Faille identifiée avec poste explicite et réutiliser une seule page de runes.
L'identifiant Riot peut manquer avant le lancement : un compteur de draft Rust
invalide les anciennes requêtes sans attendre cet identifiant.

Décision utilisateur du 3 octobre 2026 : les sorts d'invocateur entrent aussi
au périmètre de #63, via une option indépendante désactivée par défaut. Cette
décision remplace la règle antérieure « D/F inchangés » quand cette option est
activée. Le [ticket #63](https://github.com/bolitow/open-lol-companion/issues/63)
conserve la décision antérieure tant que la livraison de cette extension n’est
pas intégrée ; cette nouvelle décision est portée par la MR.

## Démarrer en développement

1. Lancer PostgreSQL et l'API selon [son guide](../services/api/README.md), sur la
   base contenant les matchs et un instantané d'agrégats publié.
2. Depuis l'environnement serveur uniquement, émettre un jeton temporaire avec
   `cargo run -q -p olc-api -- token --subject development --ttl 14400` et le
   transmettre à l'environnement du processus desktop sans le journaliser.
3. Lancer `pnpm dev` avec `OLC_API_URL=http://127.0.0.1:3030` et `OLC_API_TOKEN`.
   La clé Riot et le secret de signature restent exclusivement côté serveur.
4. Dans **Réglages → Imports au prépick**, cocher les catégories souhaitées : runes, objets, sorts.
   Pour les sorts contenant Flash, choisir **D ou F** si aucune préférence
   n’est enregistrée ; les autres catégories continuent indépendamment. Le minimum
   initial vaut **1 en dev**, **100 en build de production**, réglable de 1 à
   1 000. Les imports restent initialement désactivés dans les deux modes.
5. Dans LoL, présélectionner le champion en draft Faille classique 400/420/440.
   En personnalisée Faille, choisir d'abord le **Poste en personnalisée** dans
   l'app (par exemple Support pour Bard) : les statistiques Solo/Duo sont
   utilisées et cette source est affichée. Le verrouillage n'est pas nécessaire.
   Vérifier
   **Import confirmé par le client** et la page et les sorts dans LoL, puis le set dans les ensembles d’objets de la boutique en partie.

Les réglages sont conservés sous `olc.auto-import.preferences.v1`, sans secret.
Les réglages antérieurs sont relus avec les sorts désactivés. La préférence
`olc.flash-slot` est commune aux imports manuels et automatiques : changer D/F
dans l'un des deux panneaux actualise l'autre. Une préférence inaccessible ou
absente ne choisit jamais un emplacement implicite pour Flash.
Le panneau apparaît uniquement dans les réglages ; le moteur d'import reste
actif sur les autres pages, sans réinitialisation lors de la navigation.
L'aperçu navigateur permet de lire le panneau ; l'activation exige Tauri.
Aucune valeur spéciale n'est codée pour Bard : même parcours pour tout champion.

## Données et méthode

- Population exacte : champion local présélectionné, poste attribué, région du
  compte, vraie file, patch du catalogue embarqué, tous les rangs (`ALL`).
  Aucune substitution implicite par un autre rôle, une autre région ou patch.
- En personnalisée, le poste choisi est utilisé seulement si LoL n'en attribue
  pas ; la file statistique vaut explicitement Solo/Duo (420). Le contrôle de
  l'écriture conserve la vraie file de la personnalisée (par exemple 3100).
- Variante **valide la plus jouée**, séparément pour `runes`, `final_items` et
  `summoner_spells`. Les sorts doivent être deux identifiants distincts du
  catalogue CLASSIC. Si la paire retenue contient Flash, attendre sa préférence
  D/F sans choisir à sa place une autre paire moins jouée. Sans Flash, conserver
  l’ordre de la paire statistique, soit l’orientation D/F la plus fréquente observée
  pour cette paire ; cet ordre ne représente pas un choix D/F du joueur et
  n’enregistre pas de préférence Flash.
  Les égalités suivent l'ordre numérique des sélections. Le catalogue valide
  les identifiants et positions ; une variante invalide est ignorée.
- Le minimum concerne chaque variante, pas le total de matchs du champion.
  Sous le seuil de publication de l'API, les compteurs permettent un **taux
  observé** `victoires / parties`, explicitement accompagné de l'effectif et
  de la mention petit échantillon. Aucun classement par winrate maximal.
  Une performance absente reste absente.
- Exemple : 40 victoires sur 50 observations donnent 80 % observés, sans
  prouver un meilleur build. Les trois catégories n'ont pas de winrate combiné.
- Le set contient l'**inventaire final observé**, composants possibles inclus.
  Il ne prétend pas connaître le premier objet ni l'ordre d'achat ; les variantes
  `purchase_order` ne sont pas importées automatiquement dans ce lot.
- Chaque nouveau contexte charge la dernière publication disponible. Les calculs
  restent produits par l'agrégateur existant (commande ponctuelle ou horaire),
  pas par ce panneau. Télécharger des matchs seul ne publie pas des agrégats.
  Une nouvelle publication ne réimporte pas une page déjà appliquée.

## Écriture et confirmation

`import_selected_build` est une commande Tauri vers Rust ; React n'effectue aucun
appel réseau local. `AutoImportContext`, sélection et reçu sont miroités dans
`@olc/shared`. `draftId` est généré dans la session Rust dès l'entrée en sélection.
`gameId`, facultatif, est une chaîne pour préserver les entiers 64 bits.

Après préparation et acquisition du verrou par catégorie, Rust relit gameflow
et draft : mode, file, identité de partie si disponible, joueur local unique,
champion **présélectionné ou verrouillé**, poste. La génération de draft Rust est
revérifiée juste avant l'écriture. Si le contexte a changé, aucune écriture. Les commandes
manuelles existantes conservent leur comportement, y compris en personnalisée.
Le schéma LCU utilisé est [le dump public](https://raw.githubusercontent.com/KebsCS/lcu-and-riotclient-api/main/lcu/swagger.json), vérifié le 02/10/2026.

- Runes : valide avec les styles réels du client et réutilise **une seule page**
  `Open LoL Companion : …`, même en changeant de champion. Les autres pages ne
  sont ni modifiées ni supprimées. Création seulement si aucune page réservée
  n'existe ; un refus du client (places pleines) ou une propriété ambiguë est
  signalé sans supprimer une page personnelle.
- Objets : conserve les sets personnels et remplace uniquement l'UID réservé
  au champion/carte ; mêmes règles de forme achetable que l'import manuel (résolution par le
  catalogue, objets sans équivalent retirés, #88). Le nombre d'objets remplacés et retirés est
  affiché dans le statut « objets » du panneau (FR/EN), car l'inventaire observé diffère alors
  du set envoyé à League. Une variante vide après retrait, ou au statut boutique illisible,
  est écartée au profit de la variante éligible suivante.
- Sorts d'invocateur : lorsque leur option est activée, réutilise le moteur #15,
  normalise Flash selon D/F et ne modifie que `spell1Id`/`spell2Id` via
  `/lol-champ-select/v1/session/my-selection`. Aucune modification du skin.
  Préparation et relecture de contexte précèdent la dernière garde de génération,
  immédiatement suivie du PATCH. Verrou et cache distincts des runes/objets ;
  le verrou sorts reste commun à l'import manuel.
- Option sorts désactivée : aucun appel à leur endpoint, D/F inchangés.
- Après acceptation, relit la page courante, le set, ou les deux sorts
  locaux dans le même contexte de draft. Si la relecture échoue
  ou diverge : **accepté, non confirmé**, jamais annoncé comme équipé.
- Une tentative par draft/champion/poste/catégorie. Passage prépick → verrouillage,
  apparition de `gameId`, révisions LCU, navigation et recalcul ne répètent pas
  l'import. Le cœur Rust garde le dernier
  succès par catégorie pendant son exécution, y compris après remontage du webview.
  Un échange de champion ou de poste réarme l'import, même lors d'un retour au
  champion précédent ; les envois en vol se terminent avant la nouvelle tentative.
  Une sortie de sélection ou une déconnexion LCU complète invalide `draftId` ;
  une draft retrouvée peut donc réappliquer la même page. Après fermeture complète
  de l'app, ce cache repart vide. Une lecture momentanément absente conserve
  l'identité et les imports déjà effectués, mais bloque toute nouvelle écriture.
- Erreur ou données manquantes : reprise par bouton explicite ; aucun polling
  agressif. Un succès, même non confirmé, n'est pas renvoyé par ce bouton.
  Les ajustements manuels ultérieurs dans LoL sont conservés. Dès l’envoi au client (y compris avant réception de sa réponse),
  changer la préférence Flash ou le seuil ne renvoie pas les sorts du champion
  courant ; la préférence s’appliquera au prochain contexte à importer.
- Désactiver bloque les futurs envois ; une écriture déjà transmise au client
  ne peut pas être annulée rétroactivement.

Hors personnalisée, l'absence de poste attribué bloque l'import. Sans file
identifiée, l'app attend. Seules les personnalisées que le gameflow identifie
comme Faille (`map.id=11`, `map.gameMode=CLASSIC`) sont acceptées. Les autres
cartes/modes et le fallback multi-patch restent hors périmètre.

## Recette Windows et macOS

Tests automatisés communs : projection de l'identité sans perte, garde de
prépick/phase/file/poste/draft, conservation des pages et sets, confirmation,
annulation d'une réponse obsolète, reconnexion et envoi en vol, seuil faible,
popularité, erreurs et reprise explicite. Le code réutilise la découverte LCU
Windows/macOS existante ; aucun nouveau chemin ou appel système propre à un OS.

Sur chaque OS, vérifier dans une vraie draft : options initialement désactivées,
Bard/support en prépick puis autre champion puis retour à Bard (même page),
passage au verrouillage sans second import, modification manuelle conservée,
sorts désactivés inchangés, sorts activés avec Flash D puis F, paire sans Flash,
préférence partagée, set personnel conservé, dodge/reconnexion et nouvelle draft,
absence de données, indisponibilité API et refus de page pleine. La réussite
automatisée ne remplace pas cette recette réelle, en particulier sous Windows.

## Intégration des réglages #11

Le fournisseur partagé des réglages porte désormais la position de Flash : la page générale, les imports au prépick et le panneau manuel lisent le même état. Le choix existant sous `olc.flash-slot` est conservé. Les notifications propres à l’ancien module de préférence ne sont plus nécessaires. En cas d’échec de sauvegarde, le choix explicite reste valable pendant cette session et l’interface le signale ; au prochain démarrage, seule la valeur effectivement enregistrée revient. L’annulation des réglages ne réécrit jamais un import déjà envoyé.
