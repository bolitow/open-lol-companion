# Icône de l’application

`source.png` conserve la mascotte flamme validée sur un fond quasi noir opaque couvrant tout le carré, assombri pour le rendu dans le Dock. Aucun nom, aucune tuile arrondie dessinée, aucun biseau ni marge transparente : le fond est celui de l’icône, pas un panneau derrière la mascotte. Cela évite un second cadre lorsque le système applique son propre traitement.

Les fichiers `icon.ico` (Windows), `icon.icns` (macOS) et les PNG sont produits avec le CLI Tauri installé dans le projet. `tauri.conf.json` les référence déjà ; le tray réutilise l’icône par défaut de la fenêtre.

Depuis la racine du dépôt, régénérer dans un dossier temporaire dédié :

```sh
pnpm --filter @olc/desktop tauri icon src-tauri/icons/source.png -o ../../work/app-icon-integration/generated
```

Recopier uniquement `32x32.png`, `128x128.png`, `128x128@2x.png`, `icon.png`, `icon.ico` et `icon.icns` ici. Les exports mobiles ne sont pas utilisés.

Recompiler l’application pour actualiser l’icône embarquée. Vérifier le Dock/Finder sur macOS et l’exécutable, le raccourci et la barre des tâches sur Windows ; les OS peuvent conserver une ancienne icône en cache.
