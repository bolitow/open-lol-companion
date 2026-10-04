import {expect, it} from 'vitest';
import {defaultOverlayPreferences, validOverlayPreferences} from './preferences';

it('refuse des réglages qui feraient sortir le panneau ou produiraient une valeur non finie', () => {
    for (const invalid of [{x: .85, width: .2}, {y: 1}, {x: -.01}, {width: .09}, {width: .51}, {opacity: -.01}, {opacity: 1.1}, {monitor: -.1}, {monitor: .5}, {monitor: 16}, {x: NaN}, {width: Infinity}]) {
        expect(validOverlayPreferences({...defaultOverlayPreferences, ...invalid}), JSON.stringify(invalid)).toBe(false);
    }
});

it('accepte le cadre aux bornes, sans rendre le plein écran exclusif implicitement compatible', () => {
    expect(validOverlayPreferences({...defaultOverlayPreferences, x: .9, y: .9, width: .1, opacity: .2, monitor: 15, exclusiveFullscreen: true})).toBe(true);
    expect(validOverlayPreferences({...defaultOverlayPreferences, x: 0, y: 0, width: .5, opacity: 1})).toBe(true);
});
