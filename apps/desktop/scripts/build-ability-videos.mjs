// #4 : références publiées par Riot ; aucune vidéo n'est téléchargée dans le dépôt.
import {readFile, writeFile, rename, rm} from 'node:fs/promises';
import {resolve} from 'node:path';
import {fileURLToPath} from 'node:url';

const rosterUrl = 'https://www.leagueoflegends.com/en-us/champions/';
const slots = ['passive', 'Q', 'W', 'E', 'R'];
const mimeExtensions = {'video/mp4': 'mp4', 'video/webm': 'webm', 'image/jpeg': 'jpg'};

function validatePageUrl(value) {
  if (typeof value !== 'string' || !/^https:\/\/www\.leagueoflegends\.com\/en-us\/champions\/[a-z0-9-]+\/$/.test(value)) throw new Error(`Page Riot invalide : ${value}`);
  return value;
}

export function validateMediaUrl(value, type) {
  if (typeof value !== 'string' || !Object.hasOwn(mimeExtensions, type)) throw new Error('Type de média invalide');
  const match = /^https:\/\/lol\.dyn\.riotcdn\.net\/x\/videos\/champion-abilities\/(\d{4})\/ability_(\d{4})_([PQWER])1\.(mp4|webm|jpg)$/.exec(value);
  if (!match || match[1] !== match[2] || match[4] !== mimeExtensions[type] || Number(match[1]) < 1) throw new Error(`Média Riot invalide : ${value}`);
  return {championId: Number(match[1]), slot: match[3] === 'P' ? 'passive' : match[3]};
}

export function parseRoster(html) {
  const pages = [...new Set([...html.matchAll(/\/en-us\/champions\/([a-z0-9-]+)\//g)].map(match => `${rosterUrl}${match[1]}/`))].sort();
  if (!pages.length) throw new Error('Roster Riot absent');
  return pages;
}

export function parseOfficialPage(html, page) {
  validatePageUrl(page);
  const script = /<script\b(?=[^>]*\bid=["']__NEXT_DATA__["'])[^>]*>([\s\S]*?)<\/script>/.exec(html);
  if (!script) throw new Error(`Métadonnées Riot absentes : ${page}`);
  const data = JSON.parse(script[1]);
  const blades = data?.props?.pageProps?.page?.blades;
  if (!Array.isArray(blades)) throw new Error(`Structure Riot inconnue : ${page}`);
  const entries = [];
  for (const blade of blades) {
    if (!Array.isArray(blade.groups)) continue;
    for (const group of blade.groups) {
      const content = group?.content;
      if (content?.media?.type !== 'video' || typeof content?.subtitle !== 'string') continue;
      const slot = content.subtitle === 'PASSIVE' ? 'passive' : content.subtitle;
      if (!slots.includes(slot)) throw new Error(`Slot Riot inconnu : ${content.subtitle}`);
      const media = content.media;
      if (!Array.isArray(media.sources) || !media.sources.length) throw new Error(`Sources absentes : ${page} ${slot}`);
      const sources = media.sources.map(source => {
        if (!['video/mp4', 'video/webm'].includes(source.type)) throw new Error('Format vidéo inconnu');
        const identity = validateMediaUrl(source.src, source.type);
        if (identity.slot !== slot) throw new Error(`Slot et média incompatibles : ${page} ${slot}`);
        return {src: source.src, type: source.type};
      });
      if (new Set(sources.map(source => source.type)).size !== sources.length) throw new Error('Sources vidéo répétées');
      const championId = validateMediaUrl(sources[0].src, sources[0].type).championId;
      if (sources.some(source => validateMediaUrl(source.src, source.type).championId !== championId)) throw new Error('Identifiants champion incompatibles');
      const poster = media.thumbnail?.url ?? null;
      if (poster !== null) {
        const identity = validateMediaUrl(poster, 'image/jpeg');
        if (identity.championId !== championId || identity.slot !== slot) throw new Error('Aperçu et vidéo incompatibles');
      }
      const {width, height} = media.dimensions ?? {};
      if (![width, height].every(value => Number.isInteger(value) && value > 0 && value <= 8192)) throw new Error('Dimensions vidéo invalides');
      entries.push({id: `${championId}:${slot}`, page, poster, width, height, sources});
    }
  }
  if (entries.length !== 5 || new Set(entries.map(entry => entry.id.split(':')[0])).size !== 1 || new Set(entries.map(entry => entry.id)).size !== 5) throw new Error(`Compétences Riot incomplètes : ${page}`);
  return entries;
}

async function mapConcurrent(values, visit, concurrency = 4) {
  const result = new Array(values.length);
  let next = 0;
  await Promise.all(Array.from({length: Math.min(concurrency, values.length)}, async () => {
    while (next < values.length) {
      const index = next++;
      result[index] = await visit(values[index], index);
    }
  }));
  return result;
}

export async function buildManifest(entries, expectedChampionIds, check, checkedAt = new Date().toISOString()) {
  const expected = new Set(expectedChampionIds.flatMap(id => slots.map(slot => `${id}:${slot}`)));
  if (!expected.size || new Set(expectedChampionIds).size !== expectedChampionIds.length || entries.length !== expected.size || entries.some(entry => !expected.delete(entry.id)) || expected.size) throw new Error('Roster incomplet, inattendu ou répété ; manifeste conservé');
  const checked = await mapConcurrent(entries, async ({id, ...entry}) => {
    const sources = [];
    for (const type of ['video/mp4', 'video/webm']) {
      const source = entry.sources.find(candidate => candidate.type === type);
      if (source && await check(source.src, source.type)) { sources.push(source); break; }
    }
    const poster = entry.poster && await check(entry.poster, 'image/jpeg') ? entry.poster : null;
    return [id, {...entry, poster, sources}];
  });
  checked.sort(([left], [right]) => left.localeCompare(right, 'en', {numeric: true}));
  return {schemaVersion: 1, checkedAt, abilities: Object.fromEntries(checked)};
}

// Un 404/410 constitue une absence stable ; une panne réseau ne produit jamais un catalogue vide.
async function request(url, method = 'GET') {
  let failure;
  for (let attempt = 0; attempt < 3; attempt++) {
    try {
      const response = await fetch(url, {method, redirect: 'error', signal: AbortSignal.timeout(20_000), headers: {'User-Agent': 'OpenLoLCompanion-AbilityCatalogue/1.0'}});
      if (response.status === 404 || response.status === 410) return response;
      if (!response.ok) throw new Error(`HTTP ${response.status} : ${url}`);
      return response;
    } catch (error) {
      failure = error;
      if (attempt < 2) await new Promise(done => setTimeout(done, (attempt + 1) * 1000));
    }
  }
  throw failure;
}

async function checkMedia(url, type) {
  validateMediaUrl(url, type);
  const response = await request(url, 'HEAD');
  if (response.status === 404 || response.status === 410) return false;
  if (response.headers.get('content-type')?.split(';')[0]?.trim() !== type) throw new Error(`Type HTTP incompatible : ${url}`);
  return true;
}

export async function generate() {
  const root = new URL('../public/game-data/', import.meta.url);
  const index = JSON.parse(await readFile(new URL('champions.json', root), 'utf8'));
  const expectedChampionIds = Object.keys(index).map(Number);
  if (!expectedChampionIds.every(id => Number.isInteger(id) && id > 0)) throw new Error('Index local invalide');
  const response = await request(rosterUrl);
  if (!response.ok) throw new Error('Roster Riot indisponible');
  const pages = parseRoster(await response.text());
  if (pages.length !== expectedChampionIds.length) throw new Error(`Rosters différents : Riot ${pages.length}, local ${expectedChampionIds.length}. Mettre à jour les données statiques avant le manifeste.`);
  const parsed = await mapConcurrent(pages, async (page, index) => {
    const result = await request(page);
    if (!result.ok) throw new Error(`Page Riot indisponible : ${page}`);
    const entries = parseOfficialPage(await result.text(), page);
    if ((index + 1) % 40 === 0) console.log(`Pages : ${index + 1}/${pages.length}`);
    return entries;
  });
  let checked = 0;
  const manifest = await buildManifest(parsed.flat(), expectedChampionIds, async (url, type) => {
    const available = await checkMedia(url, type);
    if (++checked % 200 === 0) console.log(`Médias vérifiés : ${checked}`);
    return available;
  });
  // Même dossier : le renommage n'expose jamais une écriture partielle aux lecteurs.
  const target = new URL('ability-videos.json', root);
  const temporary = new URL(`.ability-videos-${process.pid}.tmp`, root);
  try {
    await writeFile(temporary, JSON.stringify(manifest, null, 2) + '\n', {flag: 'wx'});
    await rename(temporary, target);
  } finally {
    await rm(temporary, {force: true});
  }
  const values = Object.values(manifest.abilities);
  console.log(`${values.length} sorts : ${values.filter(value => value.sources[0]?.type === 'video/mp4').length} MP4, ${values.filter(value => value.sources[0]?.type === 'video/webm').length} WebM, ${values.filter(value => !value.sources.length).length} indisponibles.`);
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  generate().catch(error => {console.error(error.message); process.exitCode = 1;});
}
