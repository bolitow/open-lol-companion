# Ressources du prototype

Téléchargées le 30 septembre 2026. Les fichiers sont servis localement : aucun appel à un CDN à l’utilisation du prototype.

## League of Legends / Riot Games

Documentation officielle : https://developer.riotgames.com/docs/lol#data-dragon

- Portraits : `https://ddragon.leagueoflegends.com/cdn/16.19.1/img/champion/{champion}.png` pour Ahri, Jinx, Lux, Yasuo, Thresh, Orianna, Vi et LeeSin.
- Objets : `https://ddragon.leagueoflegends.com/cdn/16.19.1/img/item/{id}.png` pour 3089, 3020, 3157, 3135, 3118, 6655, 3006, 3031, 3085 et 3072.
- Illustration : https://ddragon.leagueoflegends.com/cdn/img/champion/splash/Ahri_0.jpg

Ces visuels sont la propriété de Riot Games et ne sont pas couverts par la licence du code de ce dépôt. L’attribution et la mention de non-affiliation figurent dans « À propos » du prototype. Aucun visuel concurrent n’est repris.

## Polices SIL Open Font License 1.1

- Sora : https://github.com/google/fonts/tree/main/ofl/sora — fichier `Sora[wght].ttf` renommé `Sora.ttf`, licence `Sora-OFL.txt`.
- Source Sans 3 : https://github.com/google/fonts/tree/main/ofl/sourcesans3 — fichier `SourceSans3[wght].ttf` renommé `SourceSans3.ttf`, licence `SourceSans3-OFL.txt`.

Les licences originales sont conservées à côté des polices. Les icônes SVG et le contour animé sont dessinés pour ce prototype.

## Ajouts du 1er octobre 2026 — draft

Même source Data Dragon 16.19.1 : portraits Ornn, Renekton, Viego, Ezreal et Leona ; illustrations de base Lux et Orianna ; objets 1056, 2003, 2055, 3364, 1026 et 1083. Les libellés français et anglais de `draft-labels.json` sont extraits de `data/fr_FR/item.json`, `data/en_US/item.json` et des fichiers `runesReforged.json` correspondants. Icônes des runes 8112, 8229, 8214, 8230, 8224, 8210 et 8237 téléchargées depuis le chemin `icon` fourni par ces données. Noms conservés tels que fournis par Riot.

Les sélections de runes/objets sont des fixtures de conception et ne constituent pas un build conseillé ou validé. La première version présentait un aperçu partiel, remplacé par les arbres ci-dessous.

## Arbres de runes et cartes — 1er octobre 2026

- `rune-trees.json` : branches Domination et Sorcellerie complètes, extraites de `https://ddragon.leagueoflegends.com/cdn/16.19.1/data/{fr_FR|en_US}/runesReforged.json` ; noms, identifiants, ordre et rangées conservés. Les autres branches ne sont pas utilisées par les fixtures de cette maquette. Aucun éclat statistique ajouté à cet affichage.
- 25 icônes `rune-{id}.png` et deux emblèmes `rune-tree-{id}.png` : `https://ddragon.leagueoflegends.com/cdn/img/` suivi du champ `icon` des données ci-dessus.
- 12 cartes `{champion}-card.jpg` : `https://ddragon.leagueoflegends.com/cdn/img/champion/loading/{champion}_0.jpg` pour Ornn, LeeSin, Ahri, Jinx, Thresh, Renekton, Viego, Yasuo, Ezreal, Leona, Lux et Orianna. Source documentée par [Riot Data Dragon](https://developer.riotgames.com/docs/lol#data-dragon).

Les arbres sont consultables, non éditables ; les six runes actives sont des fixtures locales cohérentes par rangée, sans promesse de pertinence en jeu. Aucun appel réseau à l’exécution, aucun import dans le client.
