# Checklist de conformité Riot

Référence : [Developer API Policy](https://support-developer.riotgames.com/hc/en-us/articles/22698698001939) et section 2 du [cahier des charges](../docs/cahier-des-charges.md). À vérifier pour toute fonctionnalité qui lit ou affiche des données de jeu.

- [ ] L'information affichée est visible par le joueur dans le client ou le jeu à ce moment-là (tableau des scores, minimap, écran de sélection…).
- [ ] Aucune information sur l'adversaire qui n'est pas visible (cooldowns d'ultimes, sorts, position hors vision, or exact non affiché…).
- [ ] Aucune action automatique dans le jeu (pas de clic, pas de touche simulée). Seuls les imports dans le **client** (runes, sorts, items) sont permis, et à l'initiative du joueur ou de son réglage.
- [ ] Aucune modification du jeu ni injection dans son processus : fenêtres externes uniquement.
- [ ] Aucune publicité, nulle part.
- [ ] Pas d'imitation de l'interface de Riot.
- [ ] Données statiques (icônes, noms) issues de Data Dragon / CommunityDragon, avec la mention légale Riot.
- [ ] Fonctionnalité listée dans la demande de clé de production si elle utilise l'API Riot.

Cas limites (OCR des augments, timers de reliques, différence d'or) : ouvrir une discussion dans le ticket et obtenir une validation explicite avant de fusionner.
