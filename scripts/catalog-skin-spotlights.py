#!/usr/bin/env python3
"""Inventorie tous les skins Riot et prépare les références SkinSpotlights.

Métadonnées publiques uniquement, cache local explicite, une page de recherche
par champion. Les recherches restent partielles : aucune absence n’est prouvée.
"""
import argparse
from collections import Counter
import csv
from datetime import date
import importlib.util
import json
from pathlib import Path
import re
import ssl
import time
import urllib.error
import urllib.parse

SPEC = importlib.util.spec_from_file_location('spotlight_batch', Path(__file__).with_name('update-skin-spotlights.py'))
batch = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(batch)
# Bornes conservatrices documentées dans docs/collection-skins.md et les notes Riot.
VERSION_FLOORS = {'Kaisa':'2025-11-05',
                  'Pyke':'2025-05-29', 'Aurora':'2025-01-23',
                  'Caitlyn':'2021-11-17', 'Ahri':'2023-02-05',
                  'Fiddlesticks':'2020-04-01', 'DrMundo':'2021-06-09', 'Udyr':'2022-08-24',
                  'Jax':'2023-10-11', 'LeeSin':'2024-05-01', 'Teemo':'2024-10-09',
                  'Viktor':'2024-12-11', 'AurelionSol':'2023-02-10',
                  'Skarner':'2024-04-03', 'Volibear':'2020-05-29'}

# Riot 25.08 limite ces retouches à Grand Reckoning Talon et Sahn-Uzal Mordekaiser.
# Riot 14.17 liste seulement Cowgirl, Secret Agent, Candy Cane, Crime City et Pool Party.
SKIN_VERSION_FLOORS = {91059:'2025-05-15', 82054:'2025-05-15',
                       **{skin:'2024-08-28' for skin in (21001,21003,21004,21006,21009)}}
VERSION_SCOPE_SOURCES = [
    'https://www.leagueoflegends.com/en-us/news/game-updates/patch-25-08-notes/',
    'https://www.leagueoflegends.com/en-us/news/game-updates/patch-14-17-notes/']


def version_floor(champion, skin, minimum):
    """Le plancher d'une retouche ciblée ne s'étend pas à tout le champion."""
    return max(minimum, SKIN_VERSION_FLOORS.get(skin, VERSION_FLOORS.get(champion, minimum)))


def resolve_title(title, riot, *, allow_title_variants=False):
    """Résout dans tout Riot, jamais selon l'origine d'une recherche."""
    matches=[(champion,int(skin['id'])) for champion, data in riot.items()
             for skin in batch.title_skin_matches(title, data['data'][champion]['skins'],
                                                  allow_title_variants=allow_title_variants)]
    return matches[0] if len(matches)==1 else None


def choose_entries(entries, minimum):
    """Une référence par skin et vidéo, préférence stable pour la plus récente."""
    batch.valid_date(minimum)
    selected, issues, videos={},[],set()
    for entry in sorted(entries,key=lambda e:(e['publishedAt'], e['videoId']),reverse=True):
        if entry['publishedAt']<minimum:
            issues.append({'skinId':entry['skinId'],'videoId':entry['videoId'],'reason':'too_old'})
        elif entry['skinId'] not in selected and entry['videoId'] not in videos:
            selected[entry['skinId']]=entry
            videos.add(entry['videoId'])
    return sorted(selected.values(),key=lambda e:e['skinId']),issues


def coverage_rows(riot, entries, searches, issues):
    """Une ligne par skin, y compris ceux qui n’ont aucun candidat vérifié."""
    covered={e['skinId']:e for e in entries}
    by_skin={}
    for issue in issues:
        if issue.get('skinId') is not None:
            by_skin.setdefault(issue['skinId'],[]).append(issue)
    rows=[]
    for champion,data in sorted(riot.items()):
        model=data['data'][champion]
        search=searches.get(champion,{'status':'not_searched'})
        for skin in model['skins']:
            skin_id=int(skin['id'])
            if skin_id%1000==0:
                continue
            problems=by_skin.get(skin_id,[])
            if skin.get('parentSkin') is not None:
                status='chroma_variant'
            elif skin_id in covered:
                status='referenced'
            elif any(p['reason']=='pending' for p in problems):
                status='supplier_blocked'
            elif problems and all(p['reason']=='too_old' for p in problems):
                status='outside_date_window'
            elif problems:
                status='needs_review'
            elif search['status'] in ('rate_limited','network_blocked'):
                status='supplier_blocked'
            elif search['status']=='partial':
                status='search_incomplete'
            else:
                status='not_searched'
            entry=covered.get(skin_id,{})
            rows.append({'skinId':skin_id,'championId':int(model['key']),'champion':champion,
                         'name':skin['name'],'kind':'chroma' if skin.get('parentSkin') is not None else 'skin','status':status,'videoId':entry.get('videoId',''),
                         'publishedAt':entry.get('publishedAt',''),
                         'reason':' | '.join(sorted({p['reason'] for p in problems})),
                         'candidates':' | '.join(sorted({p.get('videoId','') for p in problems})),
                         'searchStatus':search['status']})
    return rows


def search_page(raw, *, official_only=False):
    """Lit les résultats HTML publics sans exécuter les scripts ni suivre la pagination."""
    text=raw.decode('utf-8')
    marker=re.search(r'var ytInitialData\s*=\s*',text)
    if marker is None:
        raise ValueError('Métadonnées de recherche absentes')
    data=json.JSONDecoder().raw_decode(text[marker.end():])[0]
    videos={}
    def walk(node):
        if isinstance(node,dict):
            renderer=node.get('videoRenderer')
            if isinstance(renderer,dict):
                title=renderer.get('title',{})
                label=title.get('simpleText') or ''.join(r.get('text','') for r in title.get('runs',[]))
                video=renderer.get('videoId','')
                owners = {run.get('navigationEndpoint', {}).get('browseEndpoint', {}).get('browseId')
                          for run in renderer.get('ownerText', {}).get('runs', [])}
                if re.fullmatch(r'[-_a-zA-Z0-9]{11}',video) and (not official_only or owners == {batch.CHANNEL}):
                    videos[video]={'videoId':video,'title':label}
            for value in node.values(): walk(value)
        elif isinstance(node,list):
            for value in node: walk(value)
    walk(data)
    return {'videos':list(videos.values())}


class MetadataStore:
    """Cache de JSON publics seulement ; aucune reprise après refus fournisseur."""
    def __init__(self, path, previous, fetch=batch.fetch_bytes, offline=False):
        self.path,self.previous,self.fetch,self.offline=path,list(previous),fetch,offline
        self.stop_reason=None
        self.requests=0
        self.last_request=0.0
        path.mkdir(parents=True,exist_ok=True)

    def get(self, kind, key, url):
        if kind not in ('riot','video','search') or not re.fullmatch(r'[-_a-zA-Z0-9]+',key):
            raise ValueError('Clé de cache invalide')
        for folder in [self.path,*self.previous]:
            file=folder/f'{kind}-{key}.json'
            if file.exists():
                if file.stat().st_size>batch.MAX_BYTES:
                    raise ValueError('Cache trop volumineux')
                data=json.loads(file.read_text())
                if 'error' not in data:
                    return data
        if self.offline or self.stop_reason:
            raise OSError('Requête suspendue : '+(self.stop_reason or 'offline'))
        # Une requête à la fois, au plus une par seconde ; aucune tentative automatique.
        time.sleep(max(0,1.0-(time.monotonic()-self.last_request)))
        self.last_request=time.monotonic()
        self.requests+=1
        try:
            raw=self.fetch(url)
        except urllib.error.HTTPError as error:
            error.close()
            if error.code in (429,403):
                self.stop_reason=f'http_{error.code}'
            raise
        except urllib.error.URLError as error:
            if isinstance(error.reason, ssl.SSLCertVerificationError):
                self.stop_reason='tls_certificate_error'
            raise
        if kind=='video':
            full=batch.parse_page(raw)
            details=full.get('videoDetails',{})
            micro=full.get('microformat',{}).get('playerMicroformatRenderer',{})
            data={'videoDetails':{k:details.get(k) for k in ('videoId','channelId','title','lengthSeconds','shortDescription')},
                  'microformat':{'playerMicroformatRenderer':{'publishDate':micro.get('publishDate')}},
                  'playabilityStatus':{'playableInEmbed':full.get('playabilityStatus',{}).get('playableInEmbed')}}
        elif kind=='search':
            data=search_page(raw, official_only=url.startswith('https://www.youtube.com/results?'))
            data['source']=url
        else:
            data=json.loads(raw)
        batch.write_candidate(Path('/__unused_input__'),self.path/f'{kind}-{key}.json',data)
        return data


def skin_search_targets(rows, *, include_old=False):
    """Cible les skins non résolus ; les anciens sont inclus sur option explicite."""
    selected={r['skinId']:r for r in rows if r['kind']=='skin' and (r['status'] != 'referenced' if include_old else r['status'] in ('search_incomplete','needs_review'))}
    return [selected[key] for key in sorted(selected)]


def supplement_skin_searches(rows, store, *, global_search=False):
    """Une première page par skin ciblé ; jamais une preuve d’absence de vidéo."""
    results={}
    targets=skin_search_targets(rows, include_old=global_search)
    for index,row in enumerate(targets,1):
        skin=str(row['skinId'])
        source=('https://www.youtube.com/results?search_query='+urllib.parse.quote(row['name']+' SkinSpotlights')
                if global_search else 'https://www.youtube.com/@SkinSpotlights/search?query='+urllib.parse.quote(row['name']))
        try:
            result=store.get('search',('skinwide' if global_search else 'skin')+skin,source)
            results[skin]={'status':'partial','source':source,'count':len(result['videos'])}
        except (OSError,ValueError,KeyError) as error:
            results[skin]={'status':('rate_limited' if store.stop_reason in ('http_429','http_403') else 'network_blocked') if store.stop_reason else 'not_searched','source':source,'error':str(error)}
        if index%50==0:
            print(f'Recherches ciblées : {index}/{len(targets)}, réseau : {store.requests}',flush=True)
    return results


def apply_skin_searches(report, results):
    """Rend les recherches ciblées visibles dans le CSV sans masquer un refus connu."""
    report['skinSearches']=results
    for row in report['coverage']:
        targeted=results.get(str(row['skinId']),{})
        row['skinSearchStatus']=targeted.get('status','not_targeted')
        row['skinSearchSource']=targeted.get('source','')
        if row['status']=='not_searched' and row['skinSearchStatus']=='partial':
            row['status']='search_incomplete'
        if row['status'] in ('search_incomplete','not_searched') and row['skinSearchStatus'] in ('rate_limited','network_blocked','not_searched'):
            row['status']='supplier_blocked'
    report['summary']['statuses']=dict(Counter(row['status'] for row in report['coverage']))


def collect(riot, catalog, store, minimum, checked, patch):
    """Vérifie les caches et découvertes ; les échecs restent dans le rapport."""
    candidates={}
    searches={}
    for champion,data in sorted(riot.items()):
        source='https://www.youtube.com/@SkinSpotlights/search?query='+urllib.parse.quote(data['data'][champion]['name'])
        try:
            result=store.get('search',champion,source)
            searches[champion]={'status':'partial','source':source,'count':len(result['videos'])}
            for video in result['videos']:
                candidates[video['videoId']]=video
        except (OSError,ValueError,KeyError) as error:
            searches[champion]={'status':('rate_limited' if store.stop_reason in ('http_429','http_403') else 'network_blocked') if store.stop_reason else 'not_searched','source':source,'error':str(error)}
        if len(searches)%25==0:
            print(f'Recherches : {len(searches)}/{len(riot)}, réseau : {store.requests}',flush=True)
    # Les métadonnées déjà obtenues sont utiles même si leur page de recherche manque.
    for folder in [store.path,*store.previous]:
        for file in folder.glob('search-skin*.json'):
            data=json.loads(file.read_text())
            for video in data.get('videos',[]):
                candidates.setdefault(video['videoId'],video)
        for file in folder.glob('video-*.json'):
            data=json.loads(file.read_text())
            details=data.get('videoDetails',{})
            if isinstance(details.get('videoId'),str):
                candidates.setdefault(details['videoId'],{'videoId':details['videoId'],'title':details.get('title','')})
    for entry in catalog['entries']:
        candidates.setdefault(entry['videoId'],{'videoId':entry['videoId'],'title':entry['title']})
    verified=[]
    issues=[]
    discarded_titles=0
    old={e['videoId']:e for e in catalog['entries']}
    for index,(video,candidate) in enumerate(sorted(candidates.items()),1):
        title=candidate['title']
        if 'Skin Spotlight' not in title or batch.excluded_title(title):
            discarded_titles+=1
            continue
        match=resolve_title(title,riot,allow_title_variants=True)
        if match is None:
            # Les titres composites/PBE/Classic/voix ne sont pas des références de skin uniques.
            issues.append({'videoId':video,'title':candidate['title'],'reason':'unmapped_title'})
            continue
        champion,skin=match
        floor=version_floor(champion,skin,minimum)
        try:
            data=store.get('video',video,f'https://www.youtube.com/watch?v={video}')
            actual_title=data.get('videoDetails',{}).get('title')
            if resolve_title(actual_title,riot,allow_title_variants=True) != (champion,skin):
                raise ValueError('Identité différente entre recherche et métadonnées : revue requise')
            published=(data.get('microformat',{}).get('playerMicroformatRenderer',{}).get('publishDate') or '')[:10]
            if published and published<floor:
                issues.append({'skinId':skin,'videoId':video,'reason':'too_old' if published<minimum else 'before_visual_update','minimum':floor,'publishedAt':published})
                continue
            entry,notes=batch.verify_video(data,champion,riot[champion],video,floor,checked,patch,omit_invalid_chapters=True,allow_title_variants=resolve_title(title,riot) is None)
            previous=old.get(video)
            if previous and previous.get('segments') and previous.get('descriptionSha256')==entry.get('descriptionSha256') and previous.get('durationSeconds')==entry.get('durationSeconds'):
                entry['segments']=previous['segments']
                entry['segmentVerification']=previous.get('segmentVerification',entry['segmentVerification'])
            verified.append(entry)
            if notes:
                issues.append({'skinId':skin,'videoId':video,'reason':'chapters_omitted','notes':notes})
        except OSError as error:
            issues.append({'skinId':skin,'videoId':video,'reason':'pending','detail':str(error)})
            # Conserver une référence existante dans la fenêtre si sa revalidation réseau échoue.
            if video in old and old[video]['publishedAt']>=floor:
                verified.append(old[video])
        except (ValueError,KeyError,TypeError) as error:
            issues.append({'skinId':skin,'videoId':video,'reason':'verification_failed','detail':str(error)})
        if index%100==0:
            print(f'Métadonnées : {index}/{len(candidates)}, réseau : {store.requests}',flush=True)
    entries,selection_issues=choose_entries(verified,minimum)
    issues.extend(selection_issues)
    rows=coverage_rows(riot,entries,searches,issues)
    report={'checkedAt':checked,'patch':patch,'minimumPublishedAt':minimum,'versionFloors':VERSION_FLOORS,
            'skinVersionFloors':SKIN_VERSION_FLOORS,'versionScopeSources':VERSION_SCOPE_SOURCES,
            'networkRequests':store.requests,'stopReason':store.stop_reason,'searches':searches,
            'issues':issues,'coverage':rows,
            'summary':{'champions':len(riot),'inventoryRows':len(rows),'skins':sum(r['kind']=='skin' for r in rows),'chromas':sum(r['kind']=='chroma' for r in rows),'videos':len(entries),
                       'segmentedVideos':sum(bool(e.get('segments')) for e in entries),
                       'segments':sum(len(e.get('segments',[])) for e in entries),
                       'discardedNonSkinTitles':discarded_titles,
                       'statuses':dict(Counter(r['status'] for r in rows))}}
    return {'schemaVersion':1,'entries':entries},report


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--catalog',type=Path,default=batch.DEFAULT_CATALOG)
    parser.add_argument('--sources',type=Path,required=True,help='sources.json Data Dragon du catalogue desktop')
    parser.add_argument('--cache',type=Path,required=True)
    parser.add_argument('--previous-cache',type=Path,action='append',default=[])
    parser.add_argument('--minimum-date',required=True)
    parser.add_argument('--checked-at',default=date.today().isoformat())
    parser.add_argument('--output',type=Path,required=True)
    parser.add_argument('--report',type=Path,required=True)
    parser.add_argument('--coverage-csv',type=Path,required=True)
    parser.add_argument('--offline',action='store_true')
    parser.add_argument('--global-search-uncovered-skins',action='store_true',help='Recherche publique filtrée sur la chaîne officielle, pour tous les skins sans référence, anciens inclus')
    parser.add_argument('--search-uncovered-skins',action='store_true',help='Recherche ciblée supplémentaire des skins sans résultat exact ou à revoir')
    args=parser.parse_args()
    for value in (args.minimum_date,args.checked_at): batch.valid_date(value)
    sources=json.loads(args.sources.read_text())
    patch=sources['version']
    catalog=json.loads(args.catalog.read_text())
    if len({p.resolve() for p in [args.catalog,args.sources,args.output,args.report,args.coverage_csv]})!=5:
        parser.error('Entrée et sorties distinctes requises')
    urls={}
    for source in sources['sources']:
        match=re.fullmatch(r'en_US/champion/([A-Za-z0-9]+)\.json',source['key'])
        if match and source['provider']=='ddragon' and source['version']==patch:
            url=f'https://ddragon.leagueoflegends.com/cdn/{patch}/data/en_US/champion/{match[1]}.json'
            if source['url']!=url: parser.error('Source Riot incohérente')
            urls[match[1]]=url
    store=MetadataStore(args.cache,args.previous_cache,offline=args.offline)
    riot={}
    inventory_errors=[] if urls else [{'champion':'','error':'Inventaire des champions vide'}]
    for champion,url in sorted(urls.items()):
        try:
            data=store.get('riot',champion,url)
            if data.get('version')!=patch or champion not in data.get('data',{}):
                raise ValueError('Cache Riot incompatible avec le patch')
            riot[champion]=data
        except (OSError,ValueError,KeyError) as error:
            inventory_errors.append({'champion':champion,'error':str(error)})
    result,report=collect(riot,catalog,store,args.minimum_date,args.checked_at,patch)
    if (args.search_uncovered_skins or args.global_search_uncovered_skins) and not inventory_errors:
        extra=supplement_skin_searches(report['coverage'],store,global_search=args.global_search_uncovered_skins)
        result,report=collect(riot,catalog,store,args.minimum_date,args.checked_at,patch)
        apply_skin_searches(report,extra)
    report['inventoryErrors']=inventory_errors
    report['inventoryExpectedChampions']=len(urls)
    batch.write_candidate(args.catalog,args.report,report)
    args.coverage_csv.parent.mkdir(parents=True,exist_ok=True)
    with args.coverage_csv.open('w',encoding='utf-8',newline='') as stream:
        writer=csv.DictWriter(stream,lineterminator='\n',fieldnames=['skinId','championId','champion','name','kind','status','videoId','publishedAt','reason','candidates','searchStatus','skinSearchStatus','skinSearchSource'])
        writer.writeheader();writer.writerows(report['coverage'])
    if inventory_errors:
        parser.exit(1, f'Inventaire incomplet : {len(inventory_errors)} cas ; rapport écrit, candidat non écrit.\n')
    batch.write_candidate(args.catalog,args.output,result)
    print(json.dumps(report['summary'],ensure_ascii=False))


if __name__=='__main__':
    main()
