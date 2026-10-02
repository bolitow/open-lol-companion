import {expect, it, vi} from 'vitest';
import {renderToStaticMarkup} from 'react-dom/server';
import type {Friend, FriendsState} from '@olc/shared';
import {FriendRow, FriendsPanel} from './FriendsPanel';

const friend: Friend = {name: 'Local Friend', game_name: 'Riot Player', tag_line: 'EUW', platform: 'EUW1', icon_id: 1234, presence: 'online'};
const ready: FriendsState = {revision: 1, status: 'ready', items: [friend]};

it('affiche le nom réel, la présence et le compteur dans les deux langues', () => {
    for (const locale of ['fr', 'en'] as const) {
        const state: FriendsState = {...ready, items: [friend, {...friend, name: 'Other Friend', presence: 'offline'}]};
        const html = renderToStaticMarkup(<FriendsPanel locale={locale} state={state} onPlayer={() => {}}/>);
        expect(html).toContain(locale === 'fr' ? 'Amis LoL' : 'LoL friends');
        expect(html).toContain('Local Friend');
        expect(html).toContain('Other Friend');
        expect(html).toContain('Riot Player#EUW');
        expect(html).toContain(locale === 'fr' ? '1 connecté · 2 amis' : '1 online · 2 friends');
        expect(html).toContain(locale === 'fr' ? 'Hors ligne' : 'Offline');
        expect(html).toContain('role="region"');
        expect(html).toContain('tabindex="0"');
        expect(html).not.toContain('https://');
    }
});

it('ouvre le bon profil uniquement sur une ligne d’identité complète', () => {
    const onPlayer = vi.fn();
    const row = FriendRow({locale: 'fr', friend, onPlayer});
    expect(row.type).toBe('button');
    row.props.onClick();
    expect(onPlayer).toHaveBeenCalledWith({platform: 'EUW1', game_name: 'Riot Player', tag_line: 'EUW'});
    for (const value of [{platform: null}, {tag_line: null}, {game_name: null}, {platform: 'EUW'}]) {
        const neutral = FriendRow({locale: 'fr', friend: {...friend, ...value}, onPlayer});
        expect(neutral.type).toBe('div');
        expect(neutral.props.onClick).toBeUndefined();
        expect(renderToStaticMarkup(neutral)).not.toContain('<button');
    }
    expect(onPlayer).toHaveBeenCalledTimes(1);
});

it('retire les anciennes données pour chaque état non prêt, en français et en anglais', () => {
    const cases = [
        ['disconnected', 'Ouvrez le client LoL', 'Open the LoL client'],
        ['loading', 'Chargement des amis', 'Loading friends'],
        ['unavailable', 'La liste des amis est indisponible', 'The friends list is unavailable'],
    ] as const;
    for (const [status, fr, en] of cases) for (const locale of ['fr', 'en'] as const) {
        const html = renderToStaticMarkup(<FriendsPanel locale={locale} state={{...ready, status}} onPlayer={() => {}}/>);
        expect(html).toContain(locale === 'fr' ? fr : en);
        expect(html).toContain('role="status"');
        expect(html).not.toContain('Local Friend');
        expect(html).not.toContain('Riot Player');
        expect(html).not.toContain('<button');
        if (status === 'loading') expect(html).toContain('aria-busy="true"');
    }
});

it('distingue une liste vide de l’absence de connexion sans inventer d’amis', () => {
    for (const locale of ['fr', 'en'] as const) {
        const html = renderToStaticMarkup(<FriendsPanel locale={locale} state={{...ready, items: []}} onPlayer={() => {}}/>);
        expect(html).toContain(locale === 'fr' ? 'Aucun ami à afficher' : 'No friends to display');
        expect(html).toContain('role="status"');
        expect(html).not.toContain('<li');
        expect(html).not.toContain('<button');
    }
});

it('rend les cent amis sans troncature et garde les identités incomplètes sans action', () => {
    const items = Array.from({length: 100}, (_, index): Friend => ({...friend, name: `Friend ${index}`, game_name: null, presence: index % 2 ? 'offline' : 'busy'}));
    const html = renderToStaticMarkup(<FriendsPanel locale="en" state={{...ready, items}} onPlayer={() => {}}/>);
    expect(html.match(/<li[ >]/g)).toHaveLength(100);
    expect(html).toContain('Friend 99');
    expect(html).toContain('50 online · 100 friends');
    expect(html).not.toContain('<button');
    expect(html).toContain('Profile unavailable');
});

it('affiche un statut inconnu sans prétendre que l’ami est en ligne', () => {
    const html = renderToStaticMarkup(<FriendsPanel locale="fr" state={{...ready, items: [{...friend, presence: 'unknown'}]}} onPlayer={() => {}}/>);
    expect(html).toContain('Statut inconnu');
    expect(html).toContain('0 connecté · 1 ami');
});
