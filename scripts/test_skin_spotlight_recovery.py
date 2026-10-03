"""Récupération prudente des titres et recherches publiques : sans réseau."""
import copy
import importlib.util
import json
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location('coverage_recovery', Path(__file__).with_name('catalog-skin-spotlights.py'))
mod = importlib.util.module_from_spec(spec)
spec.loader.exec_module(mod)
from test_update_skin_spotlights import metadata, VIDEO, RIOT


class TitleRecovery(unittest.TestCase):
    def resolve(self, title, names):
        riot = {'Ahri': {'data': {'Ahri': {'key': '103', 'skins': names}}}}
        return mod.resolve_title(title, riot, allow_title_variants=True)

    def test_explicit_annotations_and_typography(self):
        names = [{'id': '103007', 'name': 'Arcade Ahri'}]
        for title in ['Arcade Ahri (2023 ASU) Skin Spotlight - League of Legends',
                      'Full - Arcade Ahri Skin Spotlight - League of Legends',
                      'Arcade Ahri 2023 Skin Spotlight - League of Legends',
                      'Arcade Ahri Skin Spotlight -  League of Legends',
                      'Arcade  Ahri League of Legends Skin Spotlight']:
            with self.subTest(title=title):
                self.assertEqual(self.resolve(title, names), ('Ahri', 103007))
        self.assertEqual(self.resolve('La Ilusion Ahri Skin Spotlight - League of Legends',
                         [{'id': '103001', 'name': 'La Ilusión Ahri'}]), ('Ahri', 103001))

    def test_preserve_year_edition_and_reject_ambiguous_annotation(self):
        names = [{'id': '103001', 'name': 'Prestige Arcade Ahri'},
                 {'id': '103002', 'name': 'Prestige Arcade Ahri (2022)'}]
        self.assertEqual(self.resolve('Prestige Arcade Ahri (2022) Skin Spotlight - League of Legends', names), ('Ahri', 103002))
        self.assertEqual(self.resolve('Prestige Arcade Ahri 2022 Skin Spotlight - League of Legends', names), ('Ahri', 103002))
        self.assertIsNone(self.resolve('Prestige Arcade Ahri (2022 ASU) Skin Spotlight - League of Legends', names))

    def test_no_fuzzy_alias_composite_or_chroma(self):
        names = [{'id': '103007', 'name': 'Arcade Ahri'}, {'id': '103008', 'name': 'Arcade Ahri (Ruby)', 'parentSkin': 7}]
        for title in ['Arcad Ahri Skin Spotlight - League of Legends',
                      'Arcade Ahri (Ruby) Skin Spotlight - League of Legends',
                      'Arcade Ahri (New) Skin Spotlight - League of Legends',
                      'Arcade Ahri & Other Skin Spotlight - League of Legends',
                      'Arcade Ahri Skin Spotlight - Pre-Release - League of Legends',
                      'Arcade Ahri Skin Spotlight - Wild Rift']:
            self.assertIsNone(self.resolve(title, names))
        self.assertIsNone(self.resolve('Arcade Ahri Skin Spotlight - League of Legends', names + [{'id':'103009','name':'Arcade-Ahri'}]))

    def test_metadata_still_validated_and_source_title_preserved(self):
        data = metadata(); data['videoDetails']['title'] = 'Arcade Ahri (2023 ASU) Skin Spotlight - League of Legends'
        with self.assertRaises(ValueError):
            mod.batch.verify_video(data, 'Ahri', RIOT, VIDEO, '2019-10-03', '2026-10-03', '16.19.1')
        entry, _ = mod.batch.verify_video(data, 'Ahri', RIOT, VIDEO, '2019-10-03', '2026-10-03', '16.19.1', allow_title_variants=True)
        self.assertEqual(entry['title'], data['videoDetails']['title'])
        self.assertEqual(entry['skinId'], 103007)
        data['videoDetails']['channelId'] = 'impostor'
        with self.assertRaises(ValueError):
            mod.batch.verify_video(data, 'Ahri', RIOT, VIDEO, '2019-10-03', '2026-10-03', '16.19.1', allow_title_variants=True)

    def test_old_god_is_legitimate_but_old_marker_is_not(self):
        names = [{'id': '103007', 'name': 'Old God Ahri'}]
        self.assertEqual(self.resolve('Old God Ahri Skin Spotlight - League of Legends', names), ('Ahri', 103007))
        self.assertIsNone(self.resolve('OLD Old God Ahri Skin Spotlight - League of Legends', names))
        data=metadata(); data['videoDetails']['title']='Old God Ahri Skin Spotlight - League of Legends'
        riot=copy.deepcopy(RIOT); riot['data']['Ahri']['skins']=names
        entry,_=mod.batch.verify_video(data,'Ahri',riot,VIDEO,'2019-10-03','2026-10-03','16.19.1',allow_title_variants=True)
        self.assertEqual(entry['skinId'],103007)


class ExplicitMaintenanceOption(unittest.TestCase):
    def test_batch_can_revalidate_variants_only_on_explicit_option(self):
        from test_update_skin_spotlights import page, ENTRY
        data=metadata();data['videoDetails']['title']='Arcade Ahri (2023 ASU) Skin Spotlight - League of Legends'
        entry={**ENTRY,'title':data['videoDetails']['title']}
        def fetch(url):
            return json.dumps(RIOT).encode() if 'ddragon' in url else page(data)
        _,strict=mod.batch.build_catalog({'schemaVersion':1,'entries':[entry]},[],'16.19.1','2026-10-03',fetch)
        self.assertTrue(strict['errors'])
        result,report=mod.batch.build_catalog({'schemaVersion':1,'entries':[entry]},[],'16.19.1','2026-10-03',fetch,allow_title_variants=True)
        self.assertFalse(report['errors']);self.assertEqual(result['entries'][0]['skinId'],103007)


class SearchMetadataConsistency(unittest.TestCase):
    def test_changed_title_cannot_reuse_another_skin_version_floor(self):
        import tempfile
        from test_catalog_skin_spotlights import RIOT as inventory
        data=metadata();data['videoDetails']['title']='Star Guardian Ahri Skin Spotlight - League of Legends'
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory)
            (root/'search-Ahri.json').write_text(json.dumps({'videos':[{'videoId':VIDEO,'title':'Arcade Ahri Skin Spotlight - League of Legends'}]}))
            (root/('video-'+VIDEO+'.json')).write_text(json.dumps(data))
            result,report=mod.collect(inventory,{'schemaVersion':1,'entries':[]},mod.MetadataStore(root,[],offline=True),'2019-10-03','2026-10-03','16.19.1')
            self.assertEqual(result['entries'],[])
            self.assertTrue(any(i['reason']=='verification_failed' for i in report['issues']))


class HistoricalAliases(unittest.TestCase):
    def test_alias_requires_same_id_and_current_canonical_name(self):
        names=[{'id':'245036','name':'Arcane Firelight Ekko'}]
        aliases=[{'skinId':245036,'alias':'Firelight Ekko','canonicalName':'Arcane Firelight Ekko'}]
        title='Firelight Ekko Skin Spotlight - League of Legends'
        self.assertEqual(mod.batch.title_skin_matches(title,names,allow_title_variants=True,aliases=aliases),names)
        self.assertEqual(mod.batch.title_skin_matches(title,names,aliases=aliases),[])
        for field,value in [('skinId',245037),('canonicalName','Other')]:
            bad=[{**aliases[0],field:value}]
            self.assertEqual(mod.batch.title_skin_matches(title,names,allow_title_variants=True,aliases=bad),[])
        collision=names+[{'id':'245037','name':'Firelight Ekko'}]
        self.assertEqual(len(mod.batch.title_skin_matches(title,collision,allow_title_variants=True,aliases=aliases)),2)

    def test_historical_dated_reissue_cannot_fall_back_to_original(self):
        names=[{'id':'92004','name':'Worlds 2012 Riven'},
               {'id':'92007','name':'Reignited Worlds 2012 Riven'}]
        for name in ['Championship Riven 2016','Championship Riven (2016)']:
            matches=mod.batch.title_skin_matches(name+' Skin Spotlight - League of Legends',names,allow_title_variants=True,aliases=[a for a in mod.batch.TITLE_ALIASES if a['skinId'] != 92007])
            self.assertEqual(matches,[])
        self.assertEqual(mod.batch.title_skin_matches('Championship Riven Skin Spotlight - League of Legends',names,allow_title_variants=True),[names[0]])

    def test_exact_historical_dated_name_keeps_the_reissue(self):
        names=[{'id':'92004','name':'Worlds 2012 Riven'},{'id':'92007','name':'Reignited Worlds 2012 Riven'}]
        title='Championship Riven 2016 Skin Spotlight - League of Legends'
        self.assertEqual(mod.batch.title_skin_matches(title,names,allow_title_variants=True),[names[1]])

    def test_manifest_uses_exact_historical_and_current_identifiers(self):
        aliases=mod.batch.TITLE_ALIASES
        self.assertTrue(aliases)
        for alias in aliases:
            self.assertEqual(alias['skinId'],alias['historicalSkinId'])
            self.assertEqual(alias['skinId'],alias['currentSkinId'])
            self.assertEqual(alias['alias'],alias['historicalName'])
            self.assertEqual(alias['canonicalName'],alias['currentName'])
            self.assertRegex(alias['source'],r'^https://ddragon\.leagueoflegends\.com/cdn/\d+\.\d+\.\d+/data/en_US/champion/[A-Za-z0-9]+\.json$')
            self.assertRegex(alias['sourceSha256'],r'^[0-9a-f]{64}$')


class VersionScope(unittest.TestCase):
    def test_specific_updates_do_not_exclude_other_skins(self):
        minimum='2019-10-03'
        for champion,skin in [('Talon',91020),('Mordekaiser',82006),('MissFortune',21031),('MissFortune',21005)]:
            self.assertEqual(mod.version_floor(champion,skin,minimum),minimum)
        for champion,skin in [('Talon',91059),('Mordekaiser',82054)]:
            self.assertEqual(mod.version_floor(champion,skin,minimum),'2025-05-15')
        for skin in [21001,21003,21004,21006,21009]:
            self.assertEqual(mod.version_floor('MissFortune',skin,minimum),'2024-08-28')
        self.assertEqual(mod.version_floor('Teemo',17047,minimum),'2024-10-09')
        self.assertEqual(mod.version_floor('Talon',91020,'2026-01-01'),'2026-01-01')


class GlobalDiscovery(unittest.TestCase):
    def test_official_owner_required_not_display_name(self):
        def renderer(video, channel):
            return {'videoRenderer': {'videoId': video, 'title': {'simpleText': 'Arcade Ahri Skin Spotlight - League of Legends'},
                'ownerText': {'runs': [{'text': 'SkinSpotlights', 'navigationEndpoint': {'browseEndpoint': {'browseId': channel}}}]}}}
        data=[renderer('IPU9_WRcsj4',mod.batch.CHANNEL),renderer('7lS2jAaJG8E','fake'),renderer('abcdefghijk',None)]
        raw=('var ytInitialData = '+json.dumps(data)+';').encode()
        self.assertEqual([v['videoId'] for v in mod.search_page(raw, official_only=True)['videos']], ['IPU9_WRcsj4'])

    def test_global_queries_include_old_skins_and_stop_after_supplier_refusal(self):
        import tempfile, urllib.error
        for status in [403,429]:
            with tempfile.TemporaryDirectory() as directory:
                calls=[]
                def fetch(url):
                    calls.append(url);raise urllib.error.HTTPError(url,status,'refused',None,None)
                rows=[{'skinId':103001,'kind':'skin','name':'Dynasty Ahri','status':'outside_date_window'},
                      {'skinId':103002,'kind':'skin','name':'Midnight Ahri','status':'needs_review'}]
                result=mod.supplement_skin_searches(rows,mod.MetadataStore(Path(directory),[],fetch),global_search=True)
                self.assertEqual(len(calls),1)
                self.assertEqual(calls[0],'https://www.youtube.com/results?search_query=Dynasty%20Ahri%20SkinSpotlights')
                self.assertEqual(len(result),2)
                self.assertTrue(all(r['status']=='rate_limited' for r in result.values()))

    def test_targeted_success_is_visible_without_champion_search(self):
        report={'coverage':[{'skinId':103007,'status':'not_searched'}],'summary':{}}
        mod.apply_skin_searches(report,{'103007':{'status':'partial','source':'public-query'}})
        self.assertEqual(report['coverage'][0]['status'],'search_incomplete')

    def test_all_missing_targets_include_old_skins_but_not_chromas(self):
        rows=[{'skinId':1,'kind':'skin','status':'outside_date_window'},
              {'skinId':2,'kind':'skin','status':'referenced'},
              {'skinId':3,'kind':'chroma','status':'chroma_variant'}]
        self.assertEqual(mod.skin_search_targets(rows, include_old=True), [rows[0]])

if __name__ == '__main__':
    unittest.main()
