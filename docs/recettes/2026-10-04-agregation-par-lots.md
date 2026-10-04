# Recette : agrégation par lots (#89)

Date : 4 octobre 2026. Machine : macOS (Apple Silicon), PostgreSQL 17 en conteneur local,
binaire `olc-collector` en profil `release`. Base : copie de recette réservée à cette
évolution (19 373 parties, 210 lots patch/plateforme/file, 187 030 observations de rang),
avec l'instantané publié par #109. Aucun appel Riot.

Le lancement du nouveau binaire applique la migration `0018` à cette copie. Les quatre
calculs utilisent les mêmes paramètres que l'instantané existant :
`aggregate --all-stored --min-games 30`, avec ou sans `--incremental` (en ponctuel le
complet est le défaut ; en `--watch`, c'est l'inverse depuis la décision du 4 octobre
2026, `--full` forçant le complet). Mesures de `/usr/bin/time -l` (temps écoulé,
mémoire résidente maximale).

| Calcul | Lots relus | Temps écoulé | Mémoire max |
| --- | --- | --- | --- |
| Complet | — | 298 s | 3,87 Gio |
| Par lots, premier passage | 210 / 210 | 291 s | 1,58 Gio |
| Par lots, rien de changé | 0 / 210 | 0,3 s | 35 Mio |
| Par lots, plus gros lot modifié | 1 / 210 | 112 s | 1,39 Gio |

Le dernier cas simule une nouvelle donnée dans le lot dominant (16.19, EUW1, Solo :
9 728 parties, la moitié de la base) par une mise à jour sans effet d'une ligne de
timeline, qui change son `xmin` ; aucune donnée n'est modifiée.

**Identité des publications.** Après chaque calcul, une empreinte indépendante de
l'ordre (somme des `md5` de chaque entrée, par section) et le `md5` de l'en-tête sans
filtres ont été relevés. Les quatre publications sont identiques : 210 entrées de
couverture, 72 953 groupes, 32 765 bans, 1 992 163 variantes de builds, 1 393 340
points de compétences et 7 263 778 événements d'objets. L'ordre exact des entrées dans
chaque lot est vérifié par les tests (`incremental_tests.rs`, `tests/incremental.rs`).

Écart constaté avec l'instantané préexistant : sections `groups` et en-tête différentes,
les quatre autres identiques. Cet instantané a probablement été produit avant que le
calcul du pick rate par partie (#84) soit intégré sous #109 ; il a été remplacé avant
comparaison et n'a pas pu être relu pour le confirmer.

**Correctif de l'empreinte du catalogue (revue).** Les mesures ci-dessus prenaient la
seule version du catalogue d'objets dans l'empreinte d'un lot : une même version
republiée avec d'autres fiches (`catalog --refresh` ou `--rebuild`, complément
CommunityDragon obtenu après coup) aurait laissé ses lots réutilisés avec des étapes de
builds périmées. L'empreinte reprend désormais le classement du catalogue retenu pour
le patch (version, objets complets, bottes, trinkets, transformations) ; le test
`une_republication_du_catalogue_recalcule_les_lots_de_son_patch` le vérifie. Sur la même
copie (catalogues 16.18.1 et 16.19.1 publiés), le binaire corrigé a relu les 210 lots
(nouveau binaire) en 292 s pour 1,37 Gio, puis 0 / 210 en 0,3 s pour 33 Mio ; la
publication est identique, section par section et en-tête compris, à celle qui précédait
le correctif (même empreinte indépendante de l'ordre).

**Lecture.** La mémoire n'est plus proportionnelle à la base mais au plus gros lot
(−59 % ici, où un lot porte la moitié des parties). Le temps d'un recalcul horaire
dépend des lots modifiés : tant que la collecte observe des rangs dans une région, ses
lots classés récents sont relus ; les anciens patches et les lots inactifs ne le sont
plus. Un lot dominant reste le plafond : le découper (par rôle, par exemple) demande
de modifier l'accumulateur et n'est pas fait ici.
