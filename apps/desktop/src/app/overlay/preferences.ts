import type {OverlayPreferences} from '@olc/shared';

export const defaultOverlayPreferences: OverlayPreferences = {enabled: false, exclusiveFullscreen: false, monitor: 0, x: .02, y: .18, width: .2, opacity: .9, height: 0, style:'dark', locale: 'fr'};

export function validOverlayPreferences(preferences: OverlayPreferences): boolean {
    const {monitor, x, y, width, opacity, height, style} = preferences;
    return [monitor, x, y, width, opacity, height].every(Number.isFinite)
        && Number.isInteger(monitor) && monitor >= 0 && monitor <= 15
        && x >= 0 && x <= .9 && y >= 0 && y <= .9
        && width >= .1 && width <= .5 && x + width <= 1
        && opacity >= 0 && opacity <= 1 && (height === 0 || (height >= .1 && height <= .8)) && y + height <= 1 && (style === 'dark' || style === 'solid');
}
