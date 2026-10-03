import { expect, it, vi } from 'vitest';
import { renderToStaticMarkup } from 'react-dom/server';
import { SpotlightViewerContent, acceptSpotlightState, viewerAppearance } from './SpotlightViewer';
import type { SpotlightState } from '../../../../../packages/shared/src/spotlight';
const state: SpotlightState = { revision: 3, video: { skinId: 103007, championId: 103, videoId: 'IPU9_WRcsj4', name: 'Arcade Ahri', segments: [{ kind: 'q', start: 88, end: 97 }, { kind: 'emotes', start: 27, end: 57 }] }, selected: 'q', detached: false, locale: 'fr' };
it('présente un lecteur intégré, flèches, choix actif, détachement et fermeture', () => {
    const html = renderToStaticMarkup(<SpotlightViewerContent state={state} busy={false} error={false} onAction={vi.fn()}/>);
    expect(html).toContain('Passage précédent');
    expect(html).toContain('Passage suivant');
    expect(html).toContain('Détacher');
    expect(html).toContain('Fermer le lecteur');
    expect(html).toContain('Vidéo entière');
    expect(html).toContain('aria-pressed="true"');
    expect(html).toContain('Emotes');
    expect(html).not.toContain('Rappel');
});
it('propose le rattachement dans la fenêtre détachée et traduit les erreurs', () => {
    const html = renderToStaticMarkup(<SpotlightViewerContent state={{ ...state, detached: true, locale: 'en' }} busy error onAction={vi.fn()}/>);
    expect(html).toContain('Attach to app');
    expect(html).toContain('Try again');
    expect(html).toContain('YouTube');
    expect(html).toContain('disabled');
});
it('ignore un ancien résultat reçu après fermeture ou changement de skin', () => {
    expect(acceptSpotlightState({ ...state, revision: 5, video: null }, state).video).toBeNull();
    expect(acceptSpotlightState(state, { ...state, revision: 4, selected: 'emotes' }).selected).toBe('emotes');
});
it('conserve thème clair, langue et mouvement réduit dans le shell détaché', () => {
    expect(viewerAppearance('{"theme":"light","locale":"en","motion":true}', false)).toEqual({ theme: 'light', locale: 'en', motion: 'full' });
    expect(viewerAppearance('{"theme":"light","motion":true}', true).motion).toBe('reduced');
    expect(viewerAppearance('{"motion":false}', false).motion).toBe('reduced');
});

it('signale un chargement long sans prétendre que la vidéo est indisponible', () => {
    const html = renderToStaticMarkup(<SpotlightViewerContent state={state} busy={false} error={false} mediaStatus="slow" onAction={vi.fn()}/>);
    expect(html).toContain('Le chargement du lecteur prend plus de temps que prévu');
    expect(html).toContain('Réessayer');
    expect(html).toContain('Ouvrir sur YouTube');
});
it('propose une aide explicite même si la page YouTube chargée ne lit pas le média', () => {
    const html = renderToStaticMarkup(<SpotlightViewerContent state={state} busy={false} error={false} mediaStatus="loaded" onAction={vi.fn()}/>);
    expect(html).toContain('Vidéo bloquée');
    expect(html).not.toContain('Ouverture du lecteur…');
});
