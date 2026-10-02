export const preparationCopy = {
 fr: {
  expand:'Afficher les arbres en grand', runes:'Runes', equipped:'Équipées dans LoL', browse:'Explorer les arbres', primary:'Principale', secondary:'Secondaire', shards:'Fragments',
  missingPage:'Aucune page équipée disponible. Vous pouvez explorer les runes.', incomplete:'Page incomplète dans LoL', temporary:'Page proposée par LoL', modified:'Ajustée par LoL',
  unavailableTree:'Arbre indisponible', returnPage:'Revenir aux runes équipées', unknownPage:'Certains choix sont absents de ce catalogue.', selected:'Équipée', alternative:'Alternative',
  clearSearch:'Effacer la recherche', items:'Objets', map:'Catalogue du patch', search:'Rechercher un objet…', empty:'Aucun objet trouvé.', more:'Afficher plus', loading:'Chargement du catalogue…', error:'Catalogue indisponible.', retry:'Réessayer',
  details:'Ouvrir la fiche', close:'Fermer la fiche', components:'Composants', noComponents:'Aucun composant.', unknownItem:'Composant absent du catalogue', price:'Coût total', gold:'PO',
  noDescription:'Description indisponible pour ce patch.', patch:'Catalogue', partial:'Certaines valeurs sont absentes ou non résolues dans la source.',
  catalogHint:'Catalogue à consulter ; pas encore de build conseillé.', readOnly:'Consultation uniquement',
 },
 en: {
  expand:'Expand rune trees', runes:'Runes', equipped:'Equipped in LoL', browse:'Explore trees', primary:'Primary', secondary:'Secondary', shards:'Shards',
  missingPage:'No equipped page available. You can explore the runes.', incomplete:'Incomplete page in LoL', temporary:'Page suggested by LoL', modified:'Adjusted by LoL',
  unavailableTree:'Tree unavailable', returnPage:'Return to equipped runes', unknownPage:'Some selections are missing from this catalog.', selected:'Equipped', alternative:'Alternative',
  clearSearch:'Clear search', items:'Items', map:'Patch catalog', search:'Search for an item…', empty:'No items found.', more:'Show more', loading:'Loading catalog…', error:'Catalog unavailable.', retry:'Retry',
  details:'Open details', close:'Close details', components:'Components', noComponents:'No components.', unknownItem:'Component missing from catalog', price:'Total cost', gold:'gold',
  noDescription:'Description unavailable for this patch.', patch:'Catalog', partial:'Some values are missing or unresolved in the source.',
  catalogHint:'Browse the catalog; recommended builds are not available yet.', readOnly:'Read only',
 },
} as const;
export const statLabels: Record<string, {fr:string;en:string}> = {
 health:{fr:'PV',en:'Health'}, mana:{fr:'Mana',en:'Mana'}, armor:{fr:'Armure',en:'Armor'}, magic_resistance:{fr:'Résistance magique',en:'Magic resistance'},
 attack_damage:{fr:'Dégâts d’attaque',en:'Attack damage'}, ability_power:{fr:'Puissance',en:'Ability power'}, attack_speed:{fr:'Vitesse d’attaque',en:'Attack speed'},
 critical_strike_chance:{fr:'Chances de critique',en:'Critical strike chance'}, movement_speed:{fr:'Vitesse de déplacement',en:'Move speed'},
 life_steal:{fr:'Vol de vie',en:'Life steal'}, health_regeneration:{fr:'Régénération des PV',en:'Health regeneration'}, mana_regeneration:{fr:'Régénération du mana',en:'Mana regeneration'},
 movement_speed_ratio:{fr:'Vitesse de déplacement',en:'Move speed'}, mana_regen_percent:{fr:'Régénération de mana',en:'Mana regeneration'}, health_regen_percent:{fr:'Régénération des PV',en:'Health regeneration'},
 heal_and_shield_power:{fr:'Soins et boucliers',en:'Heal and shield power'}, lethality:{fr:'Létalité',en:'Lethality'}, omnivamp:{fr:'Omnivampirisme',en:'Omnivamp'}, tenacity:{fr:'Ténacité',en:'Tenacity'},
 flat_magic_penetration:{fr:'Pénétration magique',en:'Magic penetration'}, armor_penetration_percent:{fr:'Pénétration d’armure',en:'Armor penetration'}, magic_penetration_percent:{fr:'Pénétration magique',en:'Magic penetration'}, slow_resistance:{fr:'Résistance aux ralentissements',en:'Slow resistance'},
 ability_haste:{fr:'Accélération de compétence',en:'Ability haste'},
};
