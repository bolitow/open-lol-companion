#!/usr/bin/env python3
"""Prépare un catalogue SkinSpotlights par lot, sans télécharger les vidéos.

Métadonnées des pages publiques (format non contractuel), noms Riot versionnés,
chapitres explicites. Sortie candidate distincte : revue requise avant intégration.
"""
import argparse
from concurrent.futures import ThreadPoolExecutor
from collections import Counter
from datetime import date
import hashlib
import json
import os
from pathlib import Path
import re
import tempfile
from threading import Event
import urllib.error
import urllib.request
import unicodedata

MAX_BYTES = 5 * 1024 * 1024
CHANNEL = 'UC0NwzCHb8Fg89eTB5eYX17Q'
SUFFIX = ' Skin Spotlight - League of Legends'
ROOT = Path(__file__).resolve().parents[1]
DEFAULT_CATALOG = ROOT / 'apps/desktop/public/game-data/skin-spotlights.json'
# Renommages prouvés par le même identifiant dans deux catalogues Riot versionnés.
TITLE_ALIASES = json.loads(Path(__file__).with_name('skin-spotlight-title-aliases.json').read_text())
KINDS = {
    'passive': 'passive', 'q ability': 'q', 'w ability': 'w', 'e ability': 'e',
    'r ability': 'r', 'q': 'q', 'w': 'w', 'e': 'e', 'r': 'r',
    'recall': 'recall', 'emotes': 'emotes', 'basics': 'attack',
    'basic attacks': 'attack', 'movement': 'movement', 'rip': 'death', 'death': 'death',
}


def excluded_title(title):
    """OLD est un marqueur de version, sauf dans le nom réel Old God."""
    return bool(re.search(r'\b(PBE|Wild Rift|Pre-Release|OLD(?!\s+God\b)|Comparison)\b', title, re.I))


def normalized_name(name):
    """Typographie seulement : accents, casse, espaces et ponctuation."""
    return ''.join(c for c in unicodedata.normalize('NFKD', name.casefold())
                   if c.isalnum() and not unicodedata.combining(c))


def title_skin_matches(title, skins, *, allow_title_variants=False, aliases=None):
    """Grammaire et alias Riot sourcés ; aucun rapprochement flou."""
    if not isinstance(title, str) or excluded_title(title):
        return []
    suffixes = [SUFFIX]
    if allow_title_variants:
        suffixes += [' League of Legends Skin Spotlight', ' Skin Spotlight League of Legends']
    suffix = next((ending for ending in suffixes if title.endswith(ending)), None)
    if allow_title_variants:
        variant = re.search(r'( Skin Spotlight\s+-\s+League of Legends| League of Legends Skin Spotlight| Skin Spotlight League of Legends)$', title)
        suffix = variant[0] if variant else None
    if suffix is None:
        return []
    name = title[:-len(suffix)]
    if allow_title_variants and name.startswith('Full - '):
        name = name[len('Full - '):]
    eligible = [skin for skin in skins if int(skin['id']) % 1000 != 0 and skin.get('parentSkin') is None]
    key = normalized_name if allow_title_variants else str.casefold
    skin_ids = {int(skin['id']) for skin in eligible}
    known_aliases = [alias for alias in (TITLE_ALIASES if aliases is None else aliases) if alias['skinId'] in skin_ids]
    def matching(label):
        return [skin for skin in eligible if key(skin['name']) == key(label)
                or (allow_title_variants and any(
                    alias['skinId'] == int(skin['id']) and alias['canonicalName'] == skin['name']
                    and key(alias['alias']) == key(label) for alias in known_aliases))]
    matches = matching(name)
    if matches or not allow_title_variants:
        return matches
    annotation = re.fullmatch(r'(.+?)\s+(?:\((20\d{2})(?: (?:ASU|VGU|Update))?\)|(20\d{2}))', name)
    if annotation is None:
        return []
    base, year = annotation[1], annotation[2] or annotation[3]
    # Une édition Riot datée interdit de rabattre son annotation sur l'édition originale.
    if any(key(skin['name']) == key(base + ' ' + year) for skin in eligible):
        return []
    # Ne jamais retirer une année d'un alias historique : une réédition peut
    # avoir elle-même été renommée (Championship Riven 2016, par exemple).
    return [skin for skin in eligible if key(skin['name']) == key(base)]


def valid_date(value):
    """Exige une date civile complète et canonique."""
    if not isinstance(value, str) or date.fromisoformat(value).isoformat() != value:
        raise ValueError('Date invalide')
    return value


def fetch_bytes(url):
    """Requêtes bornées, sans cookies, médias ni clé embarquée."""
    request = urllib.request.Request(url, headers={'User-Agent': 'OpenLoLCompanion-Catalog/1.0'})
    with urllib.request.urlopen(request, timeout=25) as response:
        data = response.read(MAX_BYTES + 1)
    if len(data) > MAX_BYTES:
        raise ValueError('Réponse trop volumineuse')
    return data


def parse_page(raw):
    """Décode seulement le JSON public ; aucun script de la page n’est exécuté."""
    if len(raw) > MAX_BYTES:
        raise ValueError('Page trop volumineuse')
    text = raw.decode('utf-8')
    marker = re.search(r'var ytInitialPlayerResponse\s*=\s*', text)
    if marker is None:
        raise ValueError('Métadonnées absentes : consentement, réseau ou format changé')
    data, _ = json.JSONDecoder().raw_decode(text[marker.end():])
    if not isinstance(data, dict):
        raise ValueError('Métadonnées invalides')
    return data


def parse_chapters(description, duration):
    """Ne déduit aucun segment visuel : bornes des chapitres textuels uniquement."""
    chapters = []
    for line in description.splitlines():
        match = re.match(r'^\s*(\d+:\d+(?::\d+)?)\s+(.+?)\s*$', line)
        if match is None:
            if re.match(r'^\s*\d+:', line):
                raise ValueError('Ligne de chapitre mal formée : borne inconnue')
            continue
        parts = match[1].split(':')
        if len(parts) not in (2, 3) or any(len(p) != 2 or int(p) >= 60 for p in parts[1:]):
            raise ValueError('Timestamp de chapitre invalide')
        seconds = 0
        for part in parts:
            seconds = seconds * 60 + int(part)
        if seconds >= duration or (chapters and seconds <= chapters[-1][0]):
            raise ValueError('Chapitres non ordonnés, dupliqués ou hors durée')
        label = match[2].strip()
        chapters.append((seconds, label, KINDS.get(label.casefold())))
    counts = Counter(kind for _, _, kind in chapters if kind is not None)
    segments, notes = [], []
    for index, (start, label, kind) in enumerate(chapters):
        end = chapters[index + 1][0] if index + 1 < len(chapters) else duration
        if kind is None or counts[kind] > 1:
            notes.append({'start': start, 'label': label, 'reason': 'unknown-or-mixed' if kind is None else 'duplicate-kind'})
            continue
        segments.append({'kind': kind, 'start': start, 'end': end})
    if not chapters:
        notes.append({'reason': 'no-published-chapters'})
    return segments, notes


def verify_video(data, champion, riot, video_id, minimum, checked, patch, *, omit_invalid_chapters=False, allow_title_variants=False):
    """Association exacte, chaîne officielle et garde explicite contre les versions anciennes."""
    valid_date(minimum)
    valid_date(checked)
    if not isinstance(data, dict) or any(not isinstance(data.get(key), dict)
                                         for key in ('videoDetails', 'microformat', 'playabilityStatus')):
        raise ValueError('Structure de métadonnées invalide')
    details = data['videoDetails']
    micro = data['microformat'].get('playerMicroformatRenderer')
    if not isinstance(micro, dict) or not isinstance(details.get('title'), str) or not isinstance(micro.get('publishDate'), str):
        raise ValueError('Titre ou publication absent')
    title = details.get('title', '')
    published = valid_date(micro.get('publishDate', '')[:10])
    if (details.get('videoId') != video_id or details.get('channelId') != CHANNEL
            or excluded_title(title)
            or not minimum <= published <= checked
            or data.get('playabilityStatus', {}).get('playableInEmbed') is not True):
        raise ValueError('Vidéo, chaîne, date, édition PC finale ou intégration non vérifiée')
    name = title[:-len(SUFFIX)] if title.endswith(SUFFIX) else title
    champion_data = riot['data'][champion]
    matches = title_skin_matches(title, champion_data['skins'], allow_title_variants=allow_title_variants)
    if len(matches) != 1:
        raise ValueError('Correspondance du nom Riot absente ou ambiguë')
    canonical_name = matches[0]['name']
    skin_id, champion_id = int(matches[0]['id']), int(champion_data['key'])
    if skin_id // 1000 != champion_id or skin_id % 1000 == 0 or champion_id <= 0:
        raise ValueError('Identité Riot incohérente ou apparence de base')
    duration = int(details.get('lengthSeconds', '0'))
    if not 0 < duration <= 7200:
        raise ValueError('Durée invalide')
    description = details.get('shortDescription', '')
    if not isinstance(description, str):
        raise ValueError('Description invalide')
    try:
        segments, notes = parse_chapters(description, duration)
    except ValueError as error:
        if not omit_invalid_chapters:
            raise
        # La vidéo est déjà validée ; on retire tous les raccourcis plutôt que
        # de réparer des timestamps inconnus. Le hash garde la vraie source.
        segments = []
        notes = [{'reason': 'invalid-published-chapters', 'detail': str(error)}]
    if canonical_name != name:
        notes.append({'reason': 'title-case-difference', 'titleName': name, 'riotName': canonical_name})
    source = f'https://www.youtube.com/watch?v={video_id}'
    return {
        'skinId': skin_id, 'championId': champion_id, 'name': canonical_name,
        'videoId': video_id, 'title': title, 'channelUrl': 'https://www.youtube.com/@SkinSpotlights',
        'publishedAt': published, 'checkedAt': checked, 'source': source,
        'catalogSource': f'https://ddragon.leagueoflegends.com/cdn/{patch}/data/en_US/champion/{champion}.json',
        'channelId': CHANNEL, 'verification': ('Unique Riot EN name match with explicit title grammar, typography normalization and versioned Riot alias manifest' if allow_title_variants else 'Unique Riot EN name match (case-insensitive)') + ' and official channel; public metadata checked. Playback not verified by the generator.',
        'durationSeconds': duration,
        'descriptionSha256': hashlib.sha256(description.encode()).hexdigest(),
        'chaptersSource': source,
        'segmentVerification': 'Published chapter boundaries only; unknown or duplicate labels omitted. Not frame-accurate; playback not verified by the generator.',
        'segments': segments,
    }, notes


def build_catalog(catalog, candidates, patch, checked, fetch=fetch_bytes, *, allow_title_variants=False):
    """Partage les catalogues Riot par champion et limite le travail réseau à quatre tâches."""
    valid_date(checked)
    if not re.fullmatch(r'[1-9]\d*\.\d+\.\d+', patch):
        raise ValueError('Patch explicite requis, par exemple 16.19.1')
    if catalog.get('schemaVersion') != 1 or not isinstance(catalog.get('entries'), list):
        raise ValueError('Catalogue invalide')
    existing = catalog['entries']
    if not isinstance(candidates, list) or len(existing) + len(candidates) > 10000:
        raise ValueError('Lot invalide ou trop volumineux')
    tasks, videos, skins = [], set(), set()
    for entry in existing:
        source = re.fullmatch(r'https://ddragon\.leagueoflegends\.com/cdn/\d+\.\d+\.\d+/data/en_US/champion/([A-Za-z0-9]+)\.json', entry.get('catalogSource', ''))
        if source is None or entry['skinId'] in skins or entry['videoId'] in videos:
            raise ValueError('Source Riot invalide ou doublon dans le catalogue')
        skins.add(entry['skinId'])
        videos.add(entry['videoId'])
        tasks.append((source[1], entry['videoId'], entry['publishedAt'], entry))
    for candidate in candidates:
        video = candidate['videoId']
        if video in videos:
            raise ValueError('Vidéo candidate déjà présente dans le lot')
        videos.add(video)
        tasks.append((candidate['champion'], video, candidate['minPublishedAt'], None))
    for champion, video, minimum, _ in tasks:
        if (not isinstance(champion, str) or not re.fullmatch(r'[A-Za-z0-9]+', champion)
                or not isinstance(video, str) or not re.fullmatch(r'[-_a-zA-Z0-9]{11}', video)):
            raise ValueError('Identifiant champion ou vidéo invalide')
        valid_date(minimum)
    riot_data = {}
    rate_limited = Event()
    # Un seul accès par champion, partagé par tous ses skins ; pas de cache opaque persistant.
    for champion in sorted({task[0] for task in tasks}):
        url = f'https://ddragon.leagueoflegends.com/cdn/{patch}/data/en_US/champion/{champion}.json'
        if rate_limited.is_set():
            riot_data[champion] = ValueError('Lot suspendu après HTTP 429 ; reprendre ultérieurement')
            continue
        try:
            riot_data[champion] = json.loads(fetch(url))
        except (ValueError, OSError) as error:
            if isinstance(error, urllib.error.HTTPError):
                error.close()
                if error.code == 429:
                    rate_limited.set()
            riot_data[champion] = error

    def verify(task):
        champion, video, minimum, old = task
        try:
            if rate_limited.is_set():
                raise ValueError('Lot suspendu après HTTP 429 ; reprendre ultérieurement')
            riot = riot_data[champion]
            if isinstance(riot, Exception):
                raise ValueError('Catalogue Riot indisponible')
            entry, notes = verify_video(parse_page(fetch(f'https://www.youtube.com/watch?v={video}')),
                                       champion, riot, video, minimum, checked, patch, allow_title_variants=allow_title_variants)
            if old and (entry['skinId'] != old['skinId'] or entry['championId'] != old['championId']):
                raise ValueError('Identité existante modifiée : revue nécessaire')
            if old and old.get('segments') and old['segments'] != entry['segments']:
                # Une curation antérieure reste valable seulement sur la même source textuelle et durée.
                if old.get('descriptionSha256') != entry['descriptionSha256'] or old.get('durationSeconds') != entry['durationSeconds']:
                    raise ValueError('Chapitres existants différents : référence préservée, revue nécessaire')
                entry['segments'] = old['segments']
                entry['segmentVerification'] = old.get('segmentVerification', 'Previous curated chapters retained on identical description and duration.')
                notes.append({'reason': 'previous-curation-retained'})
            return entry, notes, None
        except (ValueError, OSError, KeyError, TypeError) as error:
            # Les tâches déjà engagées finissent ; les tâches suivantes sont suspendues.
            if isinstance(error, urllib.error.HTTPError):
                error.close()
                if error.code == 429:
                    rate_limited.set()
            return old, [], str(error)

    with ThreadPoolExecutor(max_workers=4) as pool:
        checked_entries = list(pool.map(verify, tasks))
    result = {entry['skinId']: entry for entry in existing}
    report = {'checkedAt': checked, 'patch': patch, 'errors': [], 'videos': []}
    accepted = set()
    for task, (entry, notes, error) in zip(tasks, checked_entries):
        video, old = task[1], task[3]
        if error is None and entry is not None:
            skin = entry['skinId']
            if skin in accepted or (old is None and skin in result):
                error = 'Skin déjà associé : candidat rejeté'
            else:
                accepted.add(skin)
                result[skin] = entry
        if error:
            report['errors'].append({'videoId': video, 'reason': error})
        report['videos'].append({'videoId': video, 'status': 'preserved-or-rejected' if error else 'verified',
                                 'omittedChapters': notes, 'segments': len(entry.get('segments', [])) if entry else 0})
    report['summary'] = {'references': len(result), 'segmentedVideos': sum(bool(e.get('segments')) for e in result.values()),
                         'segments': sum(len(e.get('segments', [])) for e in result.values()), 'errors': len(report['errors'])}
    return {'schemaVersion': 1, 'entries': sorted(result.values(), key=lambda e: e['skinId'])}, report


def write_candidate(source, output, data):
    """Écriture atomique, jamais en place dans le catalogue qui a servi d’entrée."""
    if source.resolve() == output.resolve() or (output.exists() and source.exists() and os.path.samefile(source, output)):
        raise ValueError('La sortie doit être distincte de la source')
    output.parent.mkdir(parents=True, exist_ok=True)
    temporary = None
    try:
        with tempfile.NamedTemporaryFile(mode='w', encoding='utf-8', newline='\n', dir=output.parent,
                                         prefix=f'.{output.name}.', suffix='.tmp', delete=False) as stream:
            temporary = Path(stream.name)
            json.dump(data, stream, ensure_ascii=False, indent=2)
            stream.write('\n')
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary, output)
    finally:
        if temporary is not None:
            temporary.unlink(missing_ok=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--catalog', type=Path, default=DEFAULT_CATALOG)
    parser.add_argument('--candidates', type=Path, help='Liste JSON champion/videoId/minPublishedAt ; optionnelle')
    parser.add_argument('--allow-title-variants', action='store_true', help='Grammaire explicite, typographie et alias Riot historiques sourcés')
    parser.add_argument('--patch', required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--report', type=Path, required=True)
    args = parser.parse_args()
    try:
        paths = [args.catalog, args.output, args.report] + ([args.candidates] if args.candidates else [])
        if len({p.resolve() for p in paths}) != len(paths):
            raise ValueError('Les chemins entrée/sortie/rapport doivent être distincts')
        catalog = json.loads(args.catalog.read_text(encoding='utf-8'))
        candidates = json.loads(args.candidates.read_text(encoding='utf-8')) if args.candidates else []
        result, report = build_catalog(catalog, candidates, args.patch, date.today().isoformat(), allow_title_variants=args.allow_title_variants)
        write_candidate(args.catalog, args.report, report)
        if report['errors']:
            parser.exit(1, f"Catalogue candidat non écrit : {len(report['errors'])} cas à revoir dans {args.report}\n")
        write_candidate(args.catalog, args.output, result)
    except (ValueError, OSError, KeyError, TypeError) as error:
        parser.exit(1, f'Catalogue source inchangé : {error}\n')
    print(json.dumps(report['summary'], ensure_ascii=False))
    print(f'Catalogue candidat à relire : {args.output}')


if __name__ == '__main__':
    main()
