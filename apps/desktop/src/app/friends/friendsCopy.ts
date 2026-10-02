import type {FriendPresence} from '@olc/shared';
import type {Locale} from '../state';

interface FriendsCopy {
    title: string;
    empty: string;
    unknownFriend: string;
    openProfile: string;
    profileUnavailable: string;
    presence: Record<FriendPresence, string>;
    states: Record<'disconnected' | 'loading' | 'unavailable', string>;
    count: (online: number, total: number) => string;
}

export const friendsCopy: Record<Locale, FriendsCopy> = {
    fr: {
        title: 'Amis LoL', empty: 'Aucun ami à afficher dans le client LoL.', unknownFriend: 'Ami sans nom',
        openProfile: 'Voir le profil de', profileUnavailable: 'Profil indisponible',
        presence: {online: 'En ligne', away: 'Absent', busy: 'Occupé', offline: 'Hors ligne', unknown: 'Statut inconnu'},
        states: {
            disconnected: 'Ouvrez le client LoL et connectez-vous pour voir vos amis.',
            loading: 'Chargement des amis…',
            unavailable: 'La liste des amis est indisponible pour le moment.',
        },
        count: (online, total) => `${new Intl.NumberFormat('fr').format(online)} connecté${online > 1 ? 's' : ''} · ${new Intl.NumberFormat('fr').format(total)} ami${total > 1 ? 's' : ''}`,
    },
    en: {
        title: 'LoL friends', empty: 'No friends to display in the LoL client.', unknownFriend: 'Unnamed friend',
        openProfile: 'View profile for', profileUnavailable: 'Profile unavailable',
        presence: {online: 'Online', away: 'Away', busy: 'Busy', offline: 'Offline', unknown: 'Unknown status'},
        states: {
            disconnected: 'Open the LoL client and sign in to see your friends.',
            loading: 'Loading friends…',
            unavailable: 'The friends list is unavailable right now.',
        },
        count: (online, total) => `${new Intl.NumberFormat('en').format(online)} online · ${new Intl.NumberFormat('en').format(total)} friend${total === 1 ? '' : 's'}`,
    },
};
