import {describe, expect, it, vi} from 'vitest';
import {buildManifest, parseOfficialPage, parseRoster, validateMediaUrl} from '../../scripts/build-ability-videos.mjs';

const page = 'https://www.leagueoflegends.com/en-us/champions/ahri/';
const media = (slot: string, extension = 'mp4', champion = '0103') => `https://lol.dyn.riotcdn.net/x/videos/champion-abilities/${champion}/ability_${champion}_${slot}1.${extension}`;
const groups = () => ['P', 'Q', 'W', 'E', 'R'].map(slot => ({content: {
  subtitle: slot === 'P' ? 'PASSIVE' : slot,
  media: {type: 'video', dimensions: {width: 1056, height: 720}, thumbnail: {url: media(slot, 'jpg')},
    sources: [{src: media(slot), type: 'video/mp4'}, {src: media(slot, 'webm'), type: 'video/webm'}]},
}}));
const html = (entries = groups()) => `<script id="__NEXT_DATA__" type="application/json">${JSON.stringify({props: {pageProps: {page: {blades: [{groups: entries}]}}}})}</script>`;

describe('catalogue des vidéos Riot', () => {
  it('extrait les pages officielles sans deviner les slugs', () => {
    expect(parseRoster('<a href="/en-us/champions/ahri/">Ahri</a><a href="/en-us/champions/ahri/">bis</a>')).toEqual([page]);
    expect(() => parseRoster('<html>Maintenance</html>')).toThrow();
  });

  it('associe les cinq sorts à leurs identifiants publiés et conserve leur ratio', () => {
    const entries = parseOfficialPage(html(), page);
    expect(entries.map(entry => entry.id)).toEqual(['103:passive', '103:Q', '103:W', '103:E', '103:R']);
    expect(entries[1]).toMatchObject({page, poster: media('Q', 'jpg'), width: 1056, height: 720});
  });

  it('refuse les domaines, chemins, formats et identifiants incohérents', () => {
    for (const url of [media('Q').replace('lol.dyn.riotcdn.net', 'evil.test'), media('Q') + '?redirect=evil', media('Q').replace('ability_0103', 'ability_0001'), media('Q').replace('https:', 'http:')]) {
      expect(() => validateMediaUrl(url, 'video/mp4')).toThrow();
    }
    expect(() => validateMediaUrl(media('Q', 'webm'), 'video/mp4')).toThrow();
    const entries = groups();
    entries[0]!.content.media.sources[0]!.src = media('P', 'mp4', '0001');
    expect(() => parseOfficialPage(html(entries), page)).toThrow();
    expect(() => parseOfficialPage(html(), 'https://evil.test/champions/ahri/')).toThrow();
  });

  it('refuse une page incomplète ou des slots répétés', () => {
    expect(() => parseOfficialPage('<html></html>', page)).toThrow();
    expect(() => parseOfficialPage(html(groups().slice(1)), page)).toThrow();
    const entries = groups(); entries[4] = entries[0]!;
    expect(() => parseOfficialPage(html(entries), page)).toThrow();
  });

  it('préfère MP4 et ne valide WebM que lorsque MP4 est absent', async () => {
    const check = vi.fn(async () => true);
    const manifest = await buildManifest(parseOfficialPage(html(), page), [103], check, '2026-10-02T20:00:00.000Z');
    expect(manifest.abilities['103:Q']?.sources).toEqual([{src: media('Q'), type: 'video/mp4'}]);
    expect(check.mock.calls).toHaveLength(10);
    expect(manifest.schemaVersion).toBe(1);
  });

  it('conserve un WebM de repli et les entrées sans média disponible', async () => {
    const check = async (url: string) => !url.includes('_P1.') && !url.includes('_Q1.mp4');
    const manifest = await buildManifest(parseOfficialPage(html(), page), [103], check, '2026-10-02T20:00:00.000Z');
    expect(manifest.abilities['103:Q']?.sources).toEqual([{src: media('Q', 'webm'), type: 'video/webm'}]);
    expect(manifest.abilities['103:passive']).toMatchObject({sources: [], poster: null});
    expect(Object.keys(manifest.abilities)).toHaveLength(5);
  });

  it('échoue sans produire de manifeste lors d’une panne ou d’un roster incomplet', async () => {
    const entries = parseOfficialPage(html(), page);
    await expect(buildManifest(entries, [103, 1], async () => true)).rejects.toThrow();
    await expect(buildManifest([...entries, ...entries], [103], async () => true)).rejects.toThrow();
    await expect(buildManifest(entries, [1], async () => true)).rejects.toThrow();
    await expect(buildManifest(entries, [103], async () => {throw new Error('HTTP 503');})).rejects.toThrow('HTTP 503');
  });
});
