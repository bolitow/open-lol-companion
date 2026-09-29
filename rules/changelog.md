# Format du CHANGELOG

`CHANGELOG.md` suit [Keep a Changelog](https://keepachangelog.com/fr/1.1.0/) et le [versionnage sémantique](https://semver.org/lang/fr/).

- Chaque PR ajoute ses lignes sous `## [Non publié]`, dans la bonne rubrique : **Ajouté**, **Modifié**, **Obsolète**, **Supprimé**, **Corrigé**, **Sécurité**.
- Une ligne = un changement compréhensible par un joueur ou un contributeur, suivi du ticket : `- Détection du client LoL sur macOS (#7)`.
- À la release, `[Non publié]` devient `## [x.y.z] — AAAA-MM-JJ` et une nouvelle section vide est ouverte.
- Pas de ligne pour une modification purement interne sans effet visible (formatage, renommage) : la regrouper sous « Modifié » si elle concerne les contributeurs.
