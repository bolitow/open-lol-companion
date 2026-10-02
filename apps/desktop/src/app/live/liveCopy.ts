export const liveCopy = {
  fr: {
    title: 'En partie', player: 'Votre champion', unknownChampion: 'Champion inconnu',
    csHint: 'LoL peut transmettre les CS par paliers de 10.',
    level: 'Niveau', kda: 'Éliminations / morts / assists', cs: 'Sbires tués', time: 'Temps de jeu',
    statuses: {
      idle: 'Aucune partie en cours.', waiting: 'En attente des données de la partie…',
      unavailable: 'Les données de la partie sont temporairement indisponibles.',
      invalid: 'Les données de la partie ne peuvent pas être affichées de façon fiable.',
    },
    desktopRequired: 'Les données en partie sont accessibles dans l’application desktop.',
    connectionError: 'La connexion aux données de la partie a été interrompue.',
    contextMissing: 'Statistiques indisponibles : le champion, le poste, la région ou la file de la partie n’a pas pu être confirmé.',
    catalogLoading: 'Chargement des données du champion…', catalogError: 'Le catalogue du jeu est indisponible.',
    readOnly: 'Consultation en partie',
    customSource: 'Partie personnalisée · statistiques Solo/Duo pour le poste choisi en draft.',
  },
  en: {
    title: 'In game', player: 'Your champion', unknownChampion: 'Unknown champion',
    csHint: 'League may report CS in increments of 10.',
    level: 'Level', kda: 'Kills / deaths / assists', cs: 'Creep score', time: 'Game time',
    statuses: {
      idle: 'No game in progress.', waiting: 'Waiting for game data…',
      unavailable: 'Game data are temporarily unavailable.',
      invalid: 'Game data cannot be displayed reliably.',
    },
    desktopRequired: 'Live game data are available in the desktop application.',
    connectionError: 'The connection to live game data was interrupted.',
    contextMissing: 'Statistics unavailable: the current champion, role, region or queue could not be confirmed.',
    catalogLoading: 'Loading champion data…', catalogError: 'The game catalog is unavailable.',
    readOnly: 'In-game reference',
    customSource: 'Custom game · Solo/Duo statistics for the role chosen in champion select.',
  },
} as const;
