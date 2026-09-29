# UX/UI, overlays et i18n

## Principes

- Le joueur est **en partie ou en draft** : chaque écran doit se lire en moins de 2 secondes. Une information principale par panneau.
- Rien ne vole le focus du jeu. Les overlays sont click-through par défaut ; seule l'édition (Alt+B) les rend interactifs.
- Thème sombre par défaut, contrastes AA minimum, tailles lisibles en 1080p à 100 % comme en 4K à 200 %.
- Maquettes de référence : ticket #4 (à lier ici une fois produites).

## Overlays

- Positions et tailles en **coordonnées relatives** à la fenêtre du jeu, jamais en pixels absolus.
- Tester 1366×768, 1920×1080, 2560×1440, ultrawide, Retina et scaling Windows 125/150 %.
- Budget : < 2 % de FPS, < 3 % de CPU ; pas d'animation continue.
- Chaque overlay a : un interrupteur, un raccourci, un aperçu dans la page « Overlays ».

## i18n

- Aucune chaîne en dur dans l'interface : clés de traduction, FR et EN fournis dans le même diff.
- Clés en anglais, hiérarchiques (`draft.suggestions.title`).
- Nombres, dates et durées formatés selon la langue (`Intl`).
- Noms de champions, items et runes : ceux de Data Dragon dans la langue du joueur, jamais traduits à la main.
