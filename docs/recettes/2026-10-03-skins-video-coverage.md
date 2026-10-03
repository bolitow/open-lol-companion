# #47 — Couverture des vidéos de skins, 3 octobre 2026

## Résultat

Inventaire Riot **173 champions / 1952 skins principaux**, patch 16.19.1. Les **7032 chromas** portant `parentSkin` sont distincts et exclus des associations vidéo autonomes. Cet inventaire Data Dragon ne prétend pas égaler le total dynamique du client LCU.

Catalogue : **895 vidéos**, **812 avec passages**, **6176 passages**. 838 nouvelles références, 1 retrait(s), 0 substitution(s) par rapport aux 58 précédentes. Taille JSON : **1 535 851 octets**. Aucun média, cache de collecte ou HTML embarqué.

Côté interface, le JSON est chargé une seule fois puis partagé entre les fiches. Côté Rust, le catalogue embarqué est désérialisé à chaque ouverture du lecteur. Ce coût local augmente avec le volume ; la vidéo elle-même reste chargée à la demande depuis YouTube. Aucune mesure de latence ou de RAM de cette désérialisation n’est revendiquée.

| État des skins principaux | Nombre |
| --- | ---: |
| Version, intégration ou métadonnées à revoir | 93 |
| Candidat trop ancien | 368 |
| Référence vérifiée par métadonnées | 895 |
| Aucun titre exact dans cette recherche partielle | 596 |

[Liste complète par identifiant Riot](2026-10-03-skins-video-coverage.csv) · [Titres à rapprocher manuellement](2026-10-03-skins-video-titles-review.csv).

## Méthode et limites

Les 173 catalogues individuels ont été contrôlés contre la liste officielle [Data Dragon](https://ddragon.leagueoflegends.com/cdn/16.19.1/data/en_US/champion.json). Une première page publique de la recherche SkinSpotlights a été examinée pour chaque champion ; 885 recherches complémentaires par nom Riot de skin non résolu ont ensuite été tentées. Leurs statuts et sources figurent dans les CSV. Ces pages ne constituent pas la liste exhaustive des vidéos de la chaîne. Une recherche sans titre exact ne prouve donc pas l’absence de vidéo.

Association par nom Riot anglais exact, insensible uniquement à la casse, puis contrôles de skinId/championId, chaîne, version PC finale, date, durée et intégration. Les titres ambigus ne sont pas rapprochés automatiquement. Les bornes publiées des chapitres sont reprises ; aucun passage n’est déduit des images ou du nom du sort.

Fenêtre : vidéos publiées depuis le 3 octobre 2019, resserrée après les refontes connues [documentées dans la maintenance](../collection-skins.md#bornes-après-mises-à-jour-visuelles-ou-de-compétences). Une ancienne vidéo exclue ne retire pas le skin de la collection. Les vidéos finales publiées juste avant certaines bornes conservatrices restent des cas de revue manuelle.

Acquisition : 2254 tentatives réseau séquentielles, au plus une nouvelle requête par seconde. Arrêt fournisseur : `aucun`. Recalcul final avec le code courant : **0 requête réseau**. Les dix recherches ciblées aux métadonnées initialement absentes ont réussi après une reprise unique ; deux vidéos supplémentaires ont été vérifiées ensuite. Les deux délais réseau du premier passage avaient aussi été repris avec succès. Le magasin CA système a été utilisé sans désactiver TLS. Les caches et le rapport technique détaillé restent dans `work/spotlight-full-47` du checkout principal, hors du catalogue livré.

3701 titres hors présentation finale unique (voix, PBE, comparaisons…) sont écartés. 894 titres supplémentaires nécessitent un rapprochement manuel ; ils ne sont pas comptés comme des skins manquants distincts.

Quatre vidéos ont des chapitres publiés désordonnés : Heartsong Seraphine (`147069`), Immortal Journey Kayle (`10057`), Winterblessed Senna (`235056`), Broken Covenant Rakan (`497027`). Le média est validé, mais seuls la lecture complète et le repli externe sont proposés. Aucun timestamp n’est réparé au jugé.

## Références retirées ou remplacées

- Retrait de God-King Garen (`86013`, publication 2018-06-25) : hors fenêtre de sept ans ; skin conservé dans la collection.

## Groupes à traiter

Les CSV gardent les identifiants et candidats pour reprendre par groupe. Priorité : refus d’intégration ou métadonnées incohérentes, vidéos avant refonte à revoir, titres ambigus, puis recherches complémentaires des skins sans résultat exact. Les skins dont tous les candidats sont trop anciens restent sans vidéo conformément au périmètre accepté. La colonne `reason` regroupe les problèmes de tous les candidats du skin, y compris des alternatives écartées : sur une ligne `referenced`, elle ne signifie pas que la vidéo sélectionnée est refusée.

| Champion | Référencés | Trop anciens | À revoir | Métadonnées manquantes | Recherche partielle sans résultat |
| --- | ---: | ---: | ---: | ---: | ---: |
| Aatrox | 6 | 5 | 0 | 0 | 1 |
| Ahri | 19 | 0 | 0 | 0 | 2 |
| Akali | 10 | 5 | 0 | 0 | 6 |
| Akshan | 3 | 0 | 0 | 0 | 1 |
| Alistar | 4 | 4 | 0 | 0 | 8 |
| Ambessa | 2 | 0 | 0 | 0 | 0 |
| Amumu | 4 | 3 | 0 | 0 | 7 |
| Anivia | 4 | 3 | 0 | 0 | 6 |
| Annie | 8 | 3 | 0 | 0 | 8 |
| Aphelios | 6 | 0 | 0 | 0 | 0 |
| Ashe | 9 | 3 | 0 | 0 | 8 |
| AurelionSol | 3 | 0 | 3 | 0 | 0 |
| Aurora | 2 | 0 | 1 | 0 | 0 |
| Azir | 3 | 2 | 0 | 0 | 2 |
| Bard | 5 | 3 | 0 | 0 | 0 |
| Belveth | 4 | 0 | 0 | 0 | 0 |
| Blitzcrank | 4 | 3 | 0 | 0 | 11 |
| Brand | 12 | 1 | 0 | 0 | 0 |
| Braum | 4 | 2 | 0 | 0 | 3 |
| Briar | 3 | 0 | 0 | 0 | 0 |
| Caitlyn | 6 | 5 | 1 | 0 | 8 |
| Camille | 5 | 2 | 0 | 0 | 1 |
| Cassiopeia | 4 | 5 | 0 | 0 | 0 |
| Chogath | 4 | 2 | 0 | 0 | 5 |
| Corki | 2 | 1 | 0 | 0 | 8 |
| Darius | 7 | 3 | 0 | 0 | 5 |
| Diana | 11 | 3 | 0 | 0 | 2 |
| DrMundo | 0 | 0 | 8 | 0 | 3 |
| Draven | 6 | 4 | 0 | 0 | 4 |
| Ekko | 5 | 5 | 0 | 0 | 2 |
| Elise | 4 | 1 | 0 | 0 | 4 |
| Evelynn | 8 | 2 | 0 | 0 | 6 |
| Ezreal | 8 | 6 | 0 | 0 | 7 |
| Fiddlesticks | 3 | 2 | 0 | 0 | 7 |
| Fiora | 8 | 4 | 0 | 0 | 4 |
| Fizz | 3 | 5 | 0 | 0 | 5 |
| Galio | 4 | 3 | 0 | 0 | 4 |
| Gangplank | 3 | 3 | 0 | 0 | 6 |
| Garen | 5 | 3 | 0 | 0 | 8 |
| Gnar | 3 | 4 | 0 | 0 | 3 |
| Gragas | 4 | 1 | 0 | 0 | 9 |
| Graves | 3 | 4 | 0 | 0 | 6 |
| Gwen | 6 | 0 | 0 | 0 | 0 |
| Hecarim | 5 | 3 | 0 | 0 | 4 |
| Heimerdinger | 3 | 1 | 0 | 0 | 5 |
| Hwei | 3 | 0 | 0 | 0 | 0 |
| Illaoi | 4 | 2 | 0 | 0 | 0 |
| Irelia | 6 | 4 | 0 | 0 | 6 |
| Ivern | 3 | 2 | 0 | 0 | 1 |
| Janna | 8 | 3 | 0 | 0 | 6 |
| JarvanIV | 1 | 1 | 0 | 0 | 13 |
| Jax | 2 | 2 | 4 | 0 | 9 |
| Jayce | 6 | 4 | 0 | 0 | 2 |
| Jhin | 7 | 5 | 0 | 0 | 1 |
| Jinx | 7 | 5 | 0 | 0 | 3 |
| KSante | 2 | 0 | 0 | 0 | 1 |
| Kaisa | 0 | 4 | 9 | 0 | 2 |
| Kalista | 3 | 1 | 0 | 0 | 2 |
| Karma | 7 | 3 | 0 | 0 | 5 |
| Karthus | 4 | 1 | 0 | 0 | 5 |
| Kassadin | 4 | 1 | 0 | 0 | 4 |
| Katarina | 8 | 2 | 0 | 0 | 9 |
| Kayle | 8 | 2 | 0 | 0 | 8 |
| Kayn | 5 | 2 | 0 | 0 | 0 |
| Kennen | 3 | 2 | 0 | 0 | 5 |
| Khazix | 3 | 3 | 0 | 0 | 2 |
| Kindred | 7 | 2 | 0 | 0 | 0 |
| Kled | 2 | 2 | 0 | 0 | 0 |
| KogMaw | 5 | 1 | 0 | 0 | 9 |
| Leblanc | 14 | 0 | 0 | 0 | 0 |
| LeeSin | 2 | 0 | 16 | 0 | 1 |
| Leona | 10 | 4 | 0 | 0 | 4 |
| Lillia | 6 | 0 | 0 | 0 | 0 |
| Lissandra | 5 | 2 | 0 | 0 | 2 |
| Locke | 1 | 0 | 0 | 0 | 0 |
| Lucian | 8 | 4 | 0 | 0 | 2 |
| Lulu | 6 | 4 | 0 | 0 | 4 |
| Lux | 10 | 5 | 0 | 0 | 7 |
| Malphite | 5 | 2 | 0 | 0 | 7 |
| Malzahar | 6 | 7 | 0 | 0 | 0 |
| Maokai | 4 | 0 | 0 | 0 | 7 |
| MasterYi | 5 | 3 | 0 | 0 | 10 |
| Mel | 3 | 0 | 0 | 0 | 0 |
| Milio | 3 | 0 | 0 | 0 | 0 |
| MissFortune | 1 | 3 | 17 | 0 | 2 |
| MonkeyKing | 3 | 2 | 0 | 0 | 4 |
| Mordekaiser | 1 | 0 | 6 | 0 | 6 |
| Morgana | 7 | 3 | 0 | 0 | 7 |
| Naafiri | 4 | 0 | 0 | 0 | 0 |
| Nami | 7 | 5 | 0 | 0 | 2 |
| Nasus | 5 | 3 | 0 | 0 | 5 |
| Nautilus | 6 | 1 | 0 | 0 | 4 |
| Neeko | 5 | 2 | 0 | 0 | 2 |
| Nidalee | 15 | 0 | 0 | 0 | 1 |
| Nilah | 3 | 0 | 0 | 0 | 0 |
| Nocturne | 3 | 1 | 0 | 0 | 6 |
| Nunu | 1 | 0 | 0 | 0 | 11 |
| Olaf | 4 | 3 | 0 | 0 | 4 |
| Orianna | 5 | 3 | 0 | 0 | 5 |
| Ornn | 4 | 1 | 0 | 0 | 0 |
| Pantheon | 4 | 1 | 0 | 0 | 8 |
| Poppy | 4 | 3 | 0 | 0 | 6 |
| Pyke | 2 | 3 | 7 | 0 | 0 |
| Qiyana | 6 | 1 | 0 | 0 | 2 |
| Quinn | 7 | 0 | 0 | 0 | 0 |
| Rakan | 7 | 4 | 0 | 0 | 1 |
| Rammus | 3 | 3 | 0 | 0 | 6 |
| RekSai | 2 | 0 | 0 | 0 | 3 |
| Rell | 4 | 0 | 0 | 0 | 0 |
| Renata | 0 | 0 | 0 | 0 | 5 |
| Renekton | 5 | 2 | 0 | 0 | 8 |
| Rengar | 4 | 2 | 0 | 0 | 3 |
| Riven | 5 | 6 | 0 | 0 | 6 |
| Rumble | 2 | 1 | 0 | 0 | 3 |
| Ryze | 3 | 1 | 0 | 0 | 10 |
| Samira | 6 | 0 | 0 | 0 | 0 |
| Sejuani | 4 | 1 | 0 | 0 | 8 |
| Senna | 11 | 0 | 0 | 0 | 0 |
| Seraphine | 10 | 0 | 0 | 0 | 3 |
| Sett | 10 | 0 | 0 | 0 | 0 |
| Shaco | 8 | 2 | 0 | 0 | 6 |
| Shen | 6 | 2 | 0 | 0 | 5 |
| Shyvana | 7 | 0 | 0 | 0 | 1 |
| Singed | 3 | 3 | 0 | 0 | 6 |
| Sion | 5 | 1 | 0 | 0 | 5 |
| Sivir | 8 | 5 | 0 | 0 | 6 |
| Skarner | 2 | 0 | 4 | 0 | 0 |
| Smolder | 2 | 0 | 0 | 0 | 0 |
| Sona | 8 | 3 | 0 | 0 | 5 |
| Soraka | 15 | 3 | 0 | 0 | 0 |
| Swain | 6 | 1 | 0 | 0 | 3 |
| Sylas | 8 | 1 | 0 | 0 | 0 |
| Syndra | 7 | 4 | 0 | 0 | 2 |
| TahmKench | 4 | 3 | 0 | 0 | 0 |
| Taliyah | 2 | 2 | 0 | 0 | 2 |
| Talon | 0 | 1 | 6 | 0 | 5 |
| Taric | 3 | 4 | 0 | 0 | 0 |
| Teemo | 1 | 3 | 4 | 0 | 7 |
| Thresh | 8 | 3 | 0 | 0 | 5 |
| Tristana | 9 | 4 | 0 | 0 | 5 |
| Trundle | 3 | 1 | 0 | 0 | 4 |
| Tryndamere | 4 | 3 | 0 | 0 | 6 |
| TwistedFate | 5 | 3 | 0 | 0 | 8 |
| Twitch | 5 | 3 | 0 | 0 | 6 |
| Udyr | 1 | 0 | 5 | 0 | 0 |
| Urgot | 3 | 4 | 0 | 0 | 1 |
| Varus | 7 | 4 | 0 | 0 | 4 |
| Vayne | 8 | 5 | 0 | 0 | 6 |
| Veigar | 6 | 2 | 0 | 0 | 8 |
| Velkoz | 4 | 2 | 0 | 0 | 1 |
| Vex | 4 | 0 | 0 | 0 | 0 |
| Vi | 5 | 4 | 0 | 0 | 4 |
| Viego | 7 | 0 | 0 | 0 | 0 |
| Viktor | 1 | 1 | 2 | 0 | 4 |
| Vladimir | 6 | 8 | 0 | 0 | 0 |
| Volibear | 4 | 1 | 0 | 0 | 5 |
| Warwick | 4 | 3 | 0 | 0 | 9 |
| Xayah | 8 | 4 | 0 | 0 | 0 |
| Xerath | 5 | 1 | 0 | 0 | 3 |
| XinZhao | 12 | 0 | 0 | 0 | 0 |
| Yasuo | 11 | 4 | 0 | 0 | 3 |
| Yone | 11 | 0 | 0 | 0 | 0 |
| Yorick | 5 | 4 | 0 | 0 | 0 |
| Yunara | 2 | 0 | 0 | 0 | 0 |
| Yuumi | 9 | 1 | 0 | 0 | 0 |
| Zaahen | 1 | 0 | 0 | 0 | 0 |
| Zac | 4 | 1 | 0 | 0 | 2 |
| Zed | 7 | 4 | 0 | 0 | 3 |
| Zeri | 6 | 0 | 0 | 0 | 0 |
| Ziggs | 4 | 3 | 0 | 0 | 5 |
| Zilean | 3 | 1 | 0 | 0 | 4 |
| Zoe | 6 | 3 | 0 | 0 | 0 |
| Zyra | 8 | 4 | 0 | 0 | 0 |

## Vérification applicative

- `pnpm test` : **996 tests réussis**, 488 TypeScript et 508 Rust ; deux tests Rust ignorés par le projet.
- `pnpm lint` : typage, format et clippy à zéro avertissement.
- `python3 -m unittest discover -s scripts -p 'test_*.py'` : **43 tests réussis**. Rouges observés avant implémentation pour couverture, protection de l’export, TLS, chromas, recherche ciblée/reporting et repli des chapitres invalides ; test applicatif rouge avant copie du catalogue puis vert.
- Bundle Tauri macOS reconstruit avec le catalogue définitif : `pnpm --filter @olc/desktop tauri build --debug --bundles app`, **78,68 MiB**.
- Revue indépendante : **895 correspondances** revalidées hors réseau depuis leurs métadonnées ; identités, noms, chaînes, dates, bornes de version, durées et hash conformes, aucun doublon/chroma. Les 57 références conservées restent strictement identiques à la baseline, curation comprise. Les CSV correspondent exactement au rapport définitif.
- Recette native macOS du catalogue définitif : recherche **Soraka divine**, fiche et vidéo `l2uSSjZc4IQ` publiées en 2024 ; passage **A 1:19–1:29**, puis **E 1:41–1:55**, URL et sélection concordantes. Lecture déclenchée manuellement, images de jeu et effet du E observés ; fermeture du lecteur avec recherche et fiche conservées. Capture locale : `work/spotlight-full-47/soraka-e-native-final.png` du checkout principal, non publiée.
- Une première recette native du candidat intermédiaire avait aussi montré Akali cauchemar criminel, passage A 1:17–1:42. Cette référence est conservée dans le catalogue définitif.

Ces recettes concernent deux exemples nouvellement ajoutés, pas les 895 lectures. La fin exacte de chaque passage n’a pas été recettée. Les contrôles de métadonnées et de chapitres ne garantissent pas la lecture du média dans toutes les Webviews. La réserve de lecture intermittente Ahri de minuit et la recette Windows du lot précédent restent ouvertes ; #47 ne sera pas clôturé. Aucun code natif ni contrat Rust/interface modifié dans cette extension.

Le bundle final est laissé ouvert sur Collection / Soraka divine, lecteur fermé. Les caches, journaux et captures restent dans le dossier temporaire de recette pour reprendre les exceptions ; ils ne sont pas embarqués ni versionnés.
