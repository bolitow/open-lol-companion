# #47 — Couverture des vidéos de skins, 3 octobre 2026

## Résultat de la recherche complémentaire

Inventaire Riot **173 champions / 1 952 skins principaux**, patch 16.19.1. Les **7 032 chromas** portant `parentSkin` restent distincts et exclus des associations vidéo autonomes. Cet inventaire Data Dragon ne prétend pas égaler le total dynamique du client LCU.

Catalogue final : **981 vidéos**, **864 avec passages**, **6 571 passages**. Cette passe ajoute **86 références**, remplace **7 présentations**, ne retire aucune référence et conserve **888 entrées identiques** au catalogue de 895. Taille JSON : **1 670 416 octets** (+134 565). Aucun média, cache de collecte ou HTML embarqué.

| État des skins principaux | Nombre |
| --- | ---: |
| Référence vérifiée par métadonnées | 981 |
| Candidats hors fenêtre de publication | 659 |
| Candidats antérieurs à la version retenue, à revoir | 112 |
| Recherche partielle sans référence confirmée | 200 |
| Total sans référence | 971 |

[Liste complète par identifiant Riot](2026-10-03-skins-video-coverage.csv) · [292 titres à rapprocher manuellement](2026-10-03-skins-video-titles-review.csv). Les titres ne sont pas des skins manquants distincts.

Le JSON est chargé une seule fois côté interface et partagé entre les fiches. Côté Rust, le catalogue embarqué est désérialisé à chaque ouverture du lecteur. Ce coût local augmente avec le volume ; la vidéo reste chargée à la demande depuis YouTube. Aucune mesure de latence ou de RAM n’est revendiquée.

## Méthode et limites

Les **1 057 skins non référencés** ont tous reçu une recherche publique complémentaire, y compris ceux dont seul un candidat trop ancien avait été trouvé. Les résultats sont filtrés par l’identifiant de la chaîne SkinSpotlights, puis vérifiés à nouveau dans les métadonnées de la vidéo. Huit réponses HTML incomplètes ont réussi après une reprise unique. Les 1 057 recherches restent **partielles** : elles ne constituent pas la liste exhaustive des vidéos de la chaîne. Une recherche sans résultat confirmé ne prouve pas l’absence de vidéo.

Le rapprochement utilise les noms Riot anglais dans tout l’inventaire, une grammaire explicite des titres finaux PC et les seules différences typographiques (casse, accents, espaces, ponctuation). Les 38 anciens noms du manifeste sont prouvés par identifiants identiques dans des catalogues Data Dragon officiels. Aucun rapprochement flou, inversion de mots ou fusion de Prestige/2022/chroma. Les noms datés sont résolus avant le retrait d’annotations ; une année n’est jamais retirée pour rabattre un alias historique. Le titre réellement récupéré doit confirmer le même skin que le résultat de recherche.

Fenêtre : publication depuis **2019-10-03**, resserrée après les [refontes connues](../collection-skins.md#bornes-après-mises-à-jour-visuelles-ou-de-compétences). Les bornes de Miss Fortune, Talon et Mordekaiser s’appliquent maintenant aux skins concernés par les notes Riot, plutôt qu’à tous leurs skins. Une preuve de renommage ne certifie pas la version visuelle. Aucun délai de tolérance global avant refonte n’est ajouté. Les 112 cas à revoir sont tous antérieurs aux bornes : 51 avant mise à jour, 61 également hors fenêtre générale. Les 659 autres cas n’ont que des candidats trop anciens.

Chaque référence ajoutée ou remplacée est contrôlée : championId/skinId, chaîne, titre PC final, date, durée, intégration autorisée, hash de description et chapitres publiés. Les **93 métadonnées modifiées ont été réacquises** dans cette passe et reconstruites indépendamment sans divergence. Les chapitres inconnus ou mixtes sont ignorés ; aucun passage n’est déduit des images.

Acquisition complémentaire : **1 874 requêtes réseau séquentielles**, au plus une nouvelle requête par seconde ; aucun arrêt fournisseur, aucune métadonnée en attente dans le résultat final. Le recalcul final utilise les caches et **0 requête réseau**. TLS reste vérifié via le magasin système. Les caches, journaux et preuves restent dans `work/spotlight-recovery-47` du checkout principal, hors application. Le passage précédent avait effectué 2 254 requêtes et livré 895 références ; ces nombres sont historiques.

Les quatre descriptions publiées désordonnées du lot précédent restent en lecture complète sans raccourcis : Heartsong Seraphine (`147069`), Immortal Journey Kayle (`10057`), Winterblessed Senna (`235056`), Broken Covenant Rakan (`497027`). Aucun timestamp n’est réparé au jugé.

## Présentations remplacées

| Skin Riot | Identifiant | Ancienne vidéo | Nouvelle vidéo |
| --- | ---: | --- | --- |
| Annie-Versary | 1012 | a1mmKOpf_8E | BA6lVghCr8E |
| Arcane Fractured Jinx | 222060 | B-Ep-SyztuI | E1F8rFth6C0 |
| Astronaut Ivern | 427020 | N7E0XBS2SV4 | fAj8oh2H0Q0 |
| Shan Hai Scrolls Neeko | 518012 | 2CSvo-p24cY | UPMotHVTkJ4 |
| Bewitching Neeko | 518022 | DSa4VVJmMyk | T2P2Eh8uXf0 |
| Battle Queen Rell | 526001 | 2R6w4_icxSk | g7BEpkB6ns0 |
| Star Guardian Rell | 526010 | sUGUqkmsTR0 | Y7Bx-YuOKzo |

Le retrait historique de God-King Garen (`86013`, publication 2018) reste celui du lot précédent. Aucun nouveau retrait dans cette passe.

## Groupes à reprendre

Les CSV conservent identifiants, candidats, motifs et URLs de recherche pour une reprise par groupe. La colonne `reason` regroupe aussi les alternatives écartées : sur une ligne `referenced`, elle ne signifie pas que la vidéo choisie est refusée. Priorité aux 112 versions à revoir puis aux 200 recherches sans référence ; les 659 cas hors fenêtre restent exclus selon le périmètre accepté. La collection conserve tous les skins, même sans vidéo.

| Champion | Référencés | Trop anciens | À revoir | Recherche partielle sans référence |
| --- | ---: | ---: | ---: | ---: |
| Aatrox | 6 | 5 | 0 | 1 |
| Ahri | 20 | 0 | 0 | 1 |
| Akali | 10 | 10 | 0 | 1 |
| Akshan | 4 | 0 | 0 | 0 |
| Alistar | 4 | 11 | 0 | 1 |
| Ambessa | 2 | 0 | 0 | 0 |
| Amumu | 4 | 9 | 0 | 1 |
| Anivia | 5 | 4 | 0 | 4 |
| Annie | 18 | 0 | 0 | 1 |
| Aphelios | 6 | 0 | 0 | 0 |
| Ashe | 9 | 9 | 0 | 2 |
| AurelionSol | 3 | 0 | 3 | 0 |
| Aurora | 2 | 0 | 1 | 0 |
| Azir | 3 | 3 | 0 | 1 |
| Bard | 5 | 3 | 0 | 0 |
| Belveth | 4 | 0 | 0 | 0 |
| Blitzcrank | 6 | 9 | 0 | 3 |
| Brand | 12 | 1 | 0 | 0 |
| Braum | 4 | 5 | 0 | 0 |
| Briar | 3 | 0 | 0 | 0 |
| Caitlyn | 15 | 2 | 2 | 1 |
| Camille | 5 | 2 | 0 | 1 |
| Cassiopeia | 4 | 5 | 0 | 0 |
| Chogath | 5 | 6 | 0 | 0 |
| Corki | 2 | 2 | 0 | 7 |
| Darius | 7 | 7 | 0 | 1 |
| Diana | 16 | 0 | 0 | 0 |
| DrMundo | 0 | 0 | 9 | 2 |
| Draven | 7 | 7 | 0 | 0 |
| Ekko | 6 | 5 | 0 | 1 |
| Elise | 4 | 4 | 0 | 1 |
| Evelynn | 8 | 6 | 0 | 2 |
| Ezreal | 8 | 13 | 0 | 0 |
| Fiddlesticks | 3 | 0 | 9 | 0 |
| Fiora | 8 | 4 | 0 | 4 |
| Fizz | 3 | 9 | 0 | 1 |
| Galio | 4 | 7 | 0 | 0 |
| Gangplank | 3 | 9 | 0 | 0 |
| Garen | 5 | 10 | 0 | 1 |
| Gnar | 4 | 6 | 0 | 0 |
| Gragas | 4 | 4 | 0 | 6 |
| Graves | 3 | 4 | 0 | 6 |
| Gwen | 6 | 0 | 0 | 0 |
| Hecarim | 5 | 7 | 0 | 0 |
| Heimerdinger | 3 | 2 | 0 | 4 |
| Hwei | 3 | 0 | 0 | 0 |
| Illaoi | 4 | 2 | 0 | 0 |
| Irelia | 6 | 8 | 0 | 2 |
| Ivern | 6 | 0 | 0 | 0 |
| Janna | 8 | 7 | 0 | 2 |
| JarvanIV | 1 | 7 | 0 | 7 |
| Jax | 2 | 0 | 15 | 0 |
| Jayce | 7 | 5 | 0 | 0 |
| Jhin | 7 | 5 | 0 | 1 |
| Jinx | 8 | 6 | 0 | 1 |
| KSante | 2 | 0 | 0 | 1 |
| Kaisa | 0 | 4 | 9 | 2 |
| Kalista | 3 | 2 | 0 | 1 |
| Karma | 7 | 8 | 0 | 0 |
| Karthus | 4 | 2 | 0 | 4 |
| Kassadin | 4 | 5 | 0 | 0 |
| Katarina | 8 | 2 | 0 | 9 |
| Kayle | 8 | 8 | 0 | 2 |
| Kayn | 5 | 2 | 0 | 0 |
| Kennen | 3 | 7 | 0 | 0 |
| Khazix | 3 | 5 | 0 | 0 |
| Kindred | 7 | 2 | 0 | 0 |
| Kled | 2 | 2 | 0 | 0 |
| KogMaw | 5 | 1 | 0 | 9 |
| Leblanc | 14 | 0 | 0 | 0 |
| LeeSin | 2 | 0 | 16 | 1 |
| Leona | 10 | 8 | 0 | 0 |
| Lillia | 6 | 0 | 0 | 0 |
| Lissandra | 5 | 4 | 0 | 0 |
| Locke | 1 | 0 | 0 | 0 |
| Lucian | 8 | 6 | 0 | 0 |
| Lulu | 6 | 8 | 0 | 0 |
| Lux | 10 | 10 | 0 | 2 |
| Malphite | 6 | 8 | 0 | 0 |
| Malzahar | 6 | 7 | 0 | 0 |
| Maokai | 4 | 7 | 0 | 0 |
| MasterYi | 5 | 3 | 0 | 10 |
| Mel | 3 | 0 | 0 | 0 |
| Milio | 3 | 0 | 0 | 0 |
| MissFortune | 13 | 3 | 5 | 2 |
| MonkeyKing | 9 | 0 | 0 | 0 |
| Mordekaiser | 7 | 5 | 1 | 0 |
| Morgana | 7 | 9 | 0 | 1 |
| Naafiri | 4 | 0 | 0 | 0 |
| Nami | 7 | 7 | 0 | 0 |
| Nasus | 5 | 4 | 0 | 4 |
| Nautilus | 11 | 0 | 0 | 0 |
| Neeko | 8 | 0 | 0 | 1 |
| Nidalee | 16 | 0 | 0 | 0 |
| Nilah | 3 | 0 | 0 | 0 |
| Nocturne | 3 | 7 | 0 | 0 |
| Nunu | 1 | 1 | 0 | 10 |
| Olaf | 4 | 7 | 0 | 0 |
| Orianna | 5 | 7 | 0 | 1 |
| Ornn | 4 | 1 | 0 | 0 |
| Pantheon | 5 | 8 | 0 | 0 |
| Poppy | 4 | 8 | 0 | 1 |
| Pyke | 2 | 3 | 7 | 0 |
| Qiyana | 7 | 1 | 0 | 1 |
| Quinn | 7 | 0 | 0 | 0 |
| Rakan | 7 | 4 | 0 | 1 |
| Rammus | 3 | 4 | 0 | 5 |
| RekSai | 3 | 2 | 0 | 0 |
| Rell | 4 | 0 | 0 | 0 |
| Renata | 0 | 0 | 0 | 5 |
| Renekton | 5 | 8 | 0 | 2 |
| Rengar | 4 | 2 | 0 | 3 |
| Riven | 5 | 11 | 0 | 1 |
| Rumble | 2 | 1 | 0 | 3 |
| Ryze | 3 | 10 | 0 | 1 |
| Samira | 6 | 0 | 0 | 0 |
| Sejuani | 4 | 7 | 0 | 2 |
| Senna | 11 | 0 | 0 | 0 |
| Seraphine | 10 | 0 | 0 | 3 |
| Sett | 10 | 0 | 0 | 0 |
| Shaco | 8 | 6 | 0 | 2 |
| Shen | 6 | 2 | 0 | 5 |
| Shyvana | 8 | 0 | 0 | 0 |
| Singed | 3 | 8 | 0 | 1 |
| Sion | 5 | 5 | 0 | 1 |
| Sivir | 8 | 6 | 0 | 5 |
| Skarner | 2 | 0 | 4 | 0 |
| Smolder | 2 | 0 | 0 | 0 |
| Sona | 8 | 3 | 0 | 5 |
| Soraka | 15 | 3 | 0 | 0 |
| Swain | 6 | 4 | 0 | 0 |
| Sylas | 8 | 1 | 0 | 0 |
| Syndra | 7 | 6 | 0 | 0 |
| TahmKench | 4 | 3 | 0 | 0 |
| Taliyah | 3 | 2 | 0 | 1 |
| Talon | 5 | 1 | 1 | 5 |
| Taric | 3 | 4 | 0 | 0 |
| Teemo | 1 | 0 | 14 | 0 |
| Thresh | 8 | 6 | 0 | 2 |
| Tristana | 9 | 9 | 0 | 0 |
| Trundle | 3 | 4 | 0 | 1 |
| Tryndamere | 4 | 9 | 0 | 0 |
| TwistedFate | 5 | 5 | 0 | 6 |
| Twitch | 5 | 9 | 0 | 0 |
| Udyr | 1 | 0 | 5 | 0 |
| Urgot | 4 | 4 | 0 | 0 |
| Varus | 7 | 8 | 0 | 0 |
| Vayne | 8 | 10 | 0 | 1 |
| Veigar | 6 | 9 | 0 | 1 |
| Velkoz | 4 | 3 | 0 | 0 |
| Vex | 4 | 0 | 0 | 0 |
| Vi | 6 | 7 | 0 | 0 |
| Viego | 7 | 0 | 0 | 0 |
| Viktor | 1 | 0 | 7 | 0 |
| Vladimir | 6 | 8 | 0 | 0 |
| Volibear | 4 | 1 | 4 | 1 |
| Warwick | 5 | 9 | 0 | 2 |
| Xayah | 8 | 4 | 0 | 0 |
| Xerath | 5 | 1 | 0 | 3 |
| XinZhao | 12 | 0 | 0 | 0 |
| Yasuo | 11 | 6 | 0 | 1 |
| Yone | 11 | 0 | 0 | 0 |
| Yorick | 5 | 4 | 0 | 0 |
| Yunara | 2 | 0 | 0 | 0 |
| Yuumi | 9 | 1 | 0 | 0 |
| Zaahen | 1 | 0 | 0 | 0 |
| Zac | 4 | 3 | 0 | 0 |
| Zed | 7 | 6 | 0 | 1 |
| Zeri | 6 | 0 | 0 | 0 |
| Ziggs | 5 | 7 | 0 | 0 |
| Zilean | 3 | 4 | 0 | 1 |
| Zoe | 6 | 3 | 0 | 0 |
| Zyra | 8 | 4 | 0 | 0 |

## Validation et réserves

- Tests Python : **59 réussis**, régressions rouges puis vertes sur les rapprochements, les versions et le catalogue intégré.
- `pnpm test` : **997 réussis** (489 TypeScript, 508 Rust), deux ignorés préexistants ; `pnpm lint` et build Tauri macOS au vert.
- Revue indépendante : aucun P1/P2 restant démontré ; 93/93 entrées modifiées reconstruites à partir des métadonnées fraîches, sans divergence. SHA-256 final : `966a723c712692bc7f04abf1a6d26858e6045ee9595ac6c4247fe26a676cf7ad`.
- Recette native macOS du bundle définitif (78,82 MiB) : Firelight Ekko, flèche Emotes → Rappel (1:02–1:16), images de jeu visibles ; Draven La Ilusión, A (1:19–1:39), images de jeu visibles. À la fermeture, la fiche et la recherche restent initialement présentes. Captures privées : `work/spotlight-recovery-47/ekko-native.png` et `draven-native.png`. La fin exacte des passages n’est pas certifiée.
- Réserve de stabilité : après deux fermetures du lecteur, le contrôle suivant a trouvé une application arrêtée ou relancée sur Accueil. Aucun rapport de crash correspondant retrouvé ; la cause (lecteur, processus ou session de contrôle) n’est pas établie. Ne pas annoncer la fermeture entièrement recettée. Le code natif du lecteur n’est pas modifié dans cette passe.
- Les 981 vidéos n’ont pas toutes été visionnées. Les métadonnées ne garantissent ni la lecture dans chaque Webview, ni chaque effet visuel/audio. Le cas intermittent Ahri de minuit reste ouvert ; aucune recette manuelle Windows n’est déduite du contrôle macOS. #47 reste ouvert.

Sources : [Data Dragon 16.19.1](https://ddragon.leagueoflegends.com/cdn/16.19.1/data/en_US/champion.json), [patch 25.08](https://www.leagueoflegends.com/en-us/news/game-updates/patch-25-08-notes/), [patch 14.17](https://www.leagueoflegends.com/en-us/news/game-updates/patch-14-17-notes/), [manifeste des 38 noms historiques](../../scripts/skin-spotlight-title-aliases.json), [maintenance et bornes](../collection-skins.md).
