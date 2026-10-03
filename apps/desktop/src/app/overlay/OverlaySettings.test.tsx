import {expect, it} from 'vitest';
import {renderToStaticMarkup} from 'react-dom/server';
import {OverlaySettings} from './OverlaySettings';

it('désactive les commandes système dans le navigateur et expose la réserve plein écran en FR/EN', () => {
    for (const locale of ['fr', 'en'] as const) {
        const html = renderToStaticMarkup(<OverlaySettings locale={locale}/>);
        expect(html).toContain('<fieldset disabled=""');
        expect(html.match(/type="checkbox"/g)).toHaveLength(2);
        expect(html).not.toContain('checked=""');
        expect(html).toContain(locale === 'fr' ? 'Ouvrez l’application desktop' : 'Open the desktop app');
        expect(html).toContain(locale === 'fr' ? 'plein écran exclusif' : 'exclusive fullscreen');
    }
});
