import {describe, expect, it} from 'vitest';
import type {Friend, FriendPresence} from '@olc/shared';
import {availableFriends, friendPlayerRequest, friendRowKey} from './friendsModel';

const friend: Friend = {name: 'Ami local', game_name: 'Riot Player', tag_line: 'EUW', platform: 'EUW1', icon_id: null, presence: 'online'};

describe('profil d’un ami LoL', () => {
    it('transmet uniquement une identité Riot complète sans reprendre le nom social', () => {
        expect(friendPlayerRequest(friend)).toEqual({platform: 'EUW1', game_name: 'Riot Player', tag_line: 'EUW'});
    });
    it('reste neutre sans identité ou plateforme valides, même si le nom ressemble à un Riot ID', () => {
        const invalid: Partial<Friend>[] = [
            {game_name: null}, {tag_line: null}, {platform: null}, {platform: 'EUW'},
            {platform: 'UNKNOWN'}, {game_name: ''}, {tag_line: ' '}, {game_name: 'Hidden\nPlayer'},
            {game_name: 'Name#Other'}, {tag_line: '..'}, {game_name: 'a'.repeat(65)},
        ];
        for (const value of invalid) expect(friendPlayerRequest({...friend, name: 'Visible#EUW', ...value})).toBeNull();
    });
    it('accepte les noms Unicode validés par la recherche de profils existante', () => {
        expect(friendPlayerRequest({...friend, game_name: 'Joueur été', tag_line: 'ABC', platform: 'NA1'}))
            .toEqual({platform: 'NA1', game_name: 'Joueur été', tag_line: 'ABC'});
    });
});

it('compte les présences connectées sans considérer un statut inconnu comme en ligne', () => {
    const presences: FriendPresence[] = ['online', 'away', 'busy', 'offline', 'unknown'];
    expect(availableFriends(presences.map(presence => ({...friend, presence})))).toBe(3);
    expect(availableFriends([])).toBe(0);
});

it('conserve la clé clavier d’une identité complète quand la liste est réordonnée',()=>{
 expect(friendRowKey(friend,0)).toBe(friendRowKey(friend,9));
});
