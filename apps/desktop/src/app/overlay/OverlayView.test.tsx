import {expect, it} from 'vitest';
import {renderToStaticMarkup} from 'react-dom/server';
import type {LiveSession, OverlayState} from '@olc/shared';
import {OverlayView} from './OverlayView';
import {defaultOverlayPreferences} from './preferences';

const state: OverlayState = {material: 'solid', revision: 1, preferences: {...defaultOverlayPreferences, enabled: true}, available: true, visible: true, preview: false, editSession: null, error: null};
const session: LiveSession = {revision: 1, generation: 1, status: 'ready', context: null, game: {gameTime: 123, gameMode: 'CLASSIC', mapNumber: 11, player: {championKey: 'Bard', level: 3, kills: 1, deaths: 0, assists: 2, creepScore: 7, items: [], currentGold: null, wardScore: null, isDead: null, respawnTimer: null, abilityLevels: null, team: null, position: null}, teams: null, events: []}, postgame: null};

it('applique l’opacité en CSS seulement au fond solide, les matériaux Mac ont une opacité native', () => {
    for (const material of ['solid', 'vibrancy', 'liquidGlass'] as const) {
        const html = renderToStaticMarkup(<OverlayView state={{...state, material, preferences: {...state.preferences, opacity: 0.5}}} session={session}/>);
        expect(html).toContain(`opacity:${material === 'solid' ? '0.5' : '1'}`);
        expect(html).toContain(`data-material="${material}"`);
    }
});

it('rend uniquement un résumé passif du joueur local, sans commande ni élément interactif', () => {
    const html = renderToStaticMarkup(<OverlayView state={state} session={session}/>);
    expect(html).toContain('Bard');
    expect(html).not.toMatch(/<(button|input|select|a|form)\b/);
    expect(html).not.toContain('tabindex');
});

it('nomme l’aperçu sans fabriquer de partie ni afficher les données live à sa place', () => {
    for (const locale of ['fr', 'en'] as const) {
        const html = renderToStaticMarkup(<OverlayView state={{...state, preview: true, preferences: {...state.preferences, enabled: false, locale}}} session={session}/>);
        expect(html).toContain(locale === 'fr' ? 'Aperçu de l’overlay' : 'Overlay preview');
        expect(html).not.toContain('Bard');
        expect(html).not.toMatch(/<(button|input|select|a|form)\b/);
    }
});

it('retire le contenu dès que le moteur masque le panneau ou interdit le plein écran', () => {
    for (const hidden of [{...state, visible: false}, {...state, available: false}, {...state, preferences: {...state.preferences, enabled: false}}, {...state, preview: true, preferences: {...state.preferences, exclusiveFullscreen: true}}]) {
        expect(renderToStaticMarkup(<OverlayView state={hidden} session={session}/>)).toBe('');
    }
});

it('réserve les poignées et commandes à la session d’édition',()=>{
 const html=renderToStaticMarkup(<OverlayView state={{...state,editSession:4}} session={session}/>);
 expect(html).toContain('Déplacer');expect(html).toContain('Redimensionner');expect(html).toContain('Annuler');
});
