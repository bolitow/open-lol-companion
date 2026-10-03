/** Informations sociales visibles dans le client local, sans identifiant technique. */
export type FriendPresence = 'online' | 'away' | 'busy' | 'offline' | 'unknown';
export interface Friend {
    name: string;
    game_name: string | null;
    tag_line: string | null;
    platform: string | null;
    icon_id: number | null;
    presence: FriendPresence;
}
export interface FriendsState {
    revision: number;
    status: 'disconnected' | 'loading' | 'ready' | 'unavailable';
    items: Friend[];
}
