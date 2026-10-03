"""Régressions hors réseau du générateur de catalogue SkinSpotlights."""
import copy
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
import urllib.error

SPEC = importlib.util.spec_from_file_location('spotlights', Path(__file__).with_name('update-skin-spotlights.py'))
# Le module absent constitue le premier rouge avant implémentation.
mod = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(mod)

VIDEO = 'IPU9_WRcsj4'
CHANNEL = 'UC0NwzCHb8Fg89eTB5eYX17Q'
SOURCE = f'https://www.youtube.com/watch?v={VIDEO}'
ENTRY = dict(skinId=103007, championId=103, name='Arcade Ahri', videoId=VIDEO,
             title='Arcade Ahri Skin Spotlight - League of Legends', channelUrl='https://www.youtube.com/@SkinSpotlights',
             publishedAt='2023-02-05', checkedAt='2026-10-03', source=SOURCE,
             catalogSource='https://ddragon.leagueoflegends.com/cdn/16.19.1/data/en_US/champion/Ahri.json')
RIOT = {'data': {'Ahri': {'key': '103', 'skins': [{'id': '103007', 'name': 'Arcade Ahri'}]}}}


def metadata():
    return {'videoDetails': {'videoId': VIDEO, 'channelId': CHANNEL, 'title': ENTRY['title'],
            'lengthSeconds': '100', 'shortDescription': '00:00 Intro\n00:10 Q Ability\n00:30 W Ability\n00:50 Chromas'},
            'microformat': {'playerMicroformatRenderer': {'publishDate': '2023-02-05T10:00:00Z'}},
            'playabilityStatus': {'playableInEmbed': True}}


def page(data):
    return ('prefix var ytInitialPlayerResponse = ' + json.dumps(data) + '; suffix').encode()


class Chapters(unittest.TestCase):
    def test_borne_sur_chapitre_suivant_meme_exclu(self):
        segments, notes = mod.parse_chapters(metadata()['videoDetails']['shortDescription'], 100)
        self.assertEqual(segments, [{'kind': 'q', 'start': 10, 'end': 30}, {'kind': 'w', 'start': 30, 'end': 50}])

    def test_ambigu_et_doublon_non_inventes(self):
        segments, notes = mod.parse_chapters('00:00 E Passive\n00:10 Passive and Basics\n00:20 Q Ability\n00:30 Q Ability\n00:40 Recall', 60)
        self.assertEqual(segments, [{'kind': 'recall', 'start': 40, 'end': 60}])
        self.assertTrue(notes)

    def test_whitelist(self):
        labels = ['Emotes', 'Recall', 'Passive', 'Basics', 'Q Ability', 'W Ability', 'E Ability', 'R Ability', 'Movement', 'RIP']
        segments, _ = mod.parse_chapters('\n'.join(f'00:{i*5:02} {label}' for i, label in enumerate(labels)), 60)
        self.assertEqual([s['kind'] for s in segments], ['emotes','recall','passive','attack','q','w','e','r','movement','death'])

    def test_rejette_timeline_incoherente(self):
        for text in ['00:10 Q Ability\n00:10 W Ability', '00:30 Q Ability\n00:20 R Ability', '00:70 Q Ability', '01:00 Q Ability', '00:10 Q Ability\n00:2x W Ability\n00:30 Recall', '00:20W Ability', '00:20']:
            with self.subTest(text=text), self.assertRaises(ValueError):
                mod.parse_chapters(text, 60)

    def test_absence_de_chapitres_est_explicite(self):
        self.assertEqual(mod.parse_chapters('No chapters', 100)[0], [])


class Identity(unittest.TestCase):
    def test_metadonnees_page_et_identite_exacte(self):
        data = mod.parse_page(page(metadata()))
        entry, _ = mod.verify_video(data, 'Ahri', RIOT, VIDEO, '2023-02-05', '2026-10-03', '16.19.1')
        self.assertEqual(entry['skinId'], 103007)
        self.assertEqual(entry['segments'][0]['kind'], 'q')
        self.assertEqual(len(entry['descriptionSha256']), 64)

    def test_refuse_chaine_video_titre_et_version_inexactes(self):
        for key, value in [('channelId','other'),('videoId','7lS2jAaJG8E'),('title','Arcade Ahri Skin Spotlight - Wild Rift'),('title','Arcade Ahri Skin Spotlight - League of Legends PBE'),('lengthSeconds','7201')]:
            data=metadata();data['videoDetails'][key]=value
            with self.subTest(key=key), self.assertRaises(ValueError):
                mod.verify_video(data,'Ahri',RIOT,VIDEO,'2023-02-05','2026-10-03','16.19.1')
        for minimum in ['2024-01-01', 'invalid']:
            with self.assertRaises(ValueError):
                mod.verify_video(metadata(),'Ahri',RIOT,VIDEO,minimum,'2026-10-03','16.19.1')

    def test_refuse_embed_et_skin_ambigu(self):
        data=metadata();data['playabilityStatus']['playableInEmbed']=False
        with self.assertRaises(ValueError): mod.verify_video(data,'Ahri',RIOT,VIDEO,'2023-02-05','2026-10-03','16.19.1')
        riot=copy.deepcopy(RIOT);riot['data']['Ahri']['skins']*=2
        with self.assertRaises(ValueError): mod.verify_video(metadata(),'Ahri',riot,VIDEO,'2023-02-05','2026-10-03','16.19.1')

    def test_casse_seule_acceptee_et_nom_riot_conserve(self):
        data=metadata();data['videoDetails']['title']='ARCADE Ahri Skin Spotlight - League of Legends'
        entry, notes=mod.verify_video(data,'Ahri',RIOT,VIDEO,'2023-02-05','2026-10-03','16.19.1')
        self.assertEqual(entry['name'],'Arcade Ahri');self.assertTrue(notes)

    def test_page_absente_ou_trop_grosse(self):
        for data in [b'consent page',b'x'*(mod.MAX_BYTES+1)]:
            with self.assertRaises(ValueError):mod.parse_page(data)


class Batch(unittest.TestCase):
    def fetch(self, url):
        return json.dumps(RIOT).encode() if 'ddragon' in url else page(metadata())

    def test_lot_indexe_et_riot_charge_une_fois(self):
        calls=[]
        def fetch(url):calls.append(url);return self.fetch(url)
        result, report=mod.build_catalog({'schemaVersion':1,'entries':[ENTRY]}, [], '16.19.1','2026-10-03', fetch)
        self.assertEqual(len(result['entries']),1)
        self.assertEqual(len(result['entries'][0]['segments']),2)
        self.assertEqual(len(calls),2)
        self.assertFalse(report['errors'])

    def test_refuse_doublons_entree_et_pas_de_fusion_inexacte(self):
        with self.assertRaises(ValueError):mod.build_catalog({'schemaVersion':1,'entries':[ENTRY,ENTRY]}, [],'16.19.1','2026-10-03',self.fetch)
        candidate={'champion':'Ahri','videoId':'7lS2jAaJG8E','minPublishedAt':'2023-02-05'}
        def fetch(url):
            if '7lS2' in url:
                data=metadata();data['videoDetails']['videoId']='7lS2jAaJG8E';return page(data)
            return self.fetch(url)
        result, report=mod.build_catalog({'schemaVersion':1,'entries':[ENTRY]},[candidate],'16.19.1','2026-10-03',fetch)
        self.assertEqual(len(result['entries']),1)
        self.assertTrue(report['errors'])

    def test_conserve_reference_si_reseau_en_echec(self):
        def fail(url):raise OSError('network')
        result, report=mod.build_catalog({'schemaVersion':1,'entries':[ENTRY]}, [],'16.19.1','2026-10-03',fail)
        self.assertEqual(result['entries'],[ENTRY]);self.assertTrue(report['errors'])

    def test_ne_remplace_pas_chapitres_valides_par_autre_timeline(self):
        old={**ENTRY,'segments':[{'kind':'q','start':12,'end':29}]}
        result, report=mod.build_catalog({'schemaVersion':1,'entries':[old]}, [],'16.19.1','2026-10-03',self.fetch)
        self.assertEqual(result['entries'],[old]);self.assertTrue(report['errors'])

    def test_sous_objets_invalides_preservent_catalogue_et_rapport(self):
        for key, value in [('videoDetails',None),('microformat',[]),('playabilityStatus',None)]:
            data=metadata();data[key]=value
            def fetch(url):return json.dumps(RIOT).encode() if 'ddragon' in url else page(data)
            result, report=mod.build_catalog({'schemaVersion':1,'entries':[ENTRY]}, [],'16.19.1','2026-10-03',fetch)
            self.assertEqual(result['entries'],[ENTRY]);self.assertTrue(report['errors'])

    def test_curation_preexistante_conservee_uniquement_si_description_identique(self):
        data=metadata();data['videoDetails']['shortDescription']='00:00 Splash\n00:10 Homeguard\n00:20 Chromas'
        fresh,_=mod.verify_video(data,'Ahri',RIOT,VIDEO,'2023-02-05','2026-10-03','16.19.1')
        old={**fresh,'segments':[{'kind':'movement','start':10,'end':20}]}
        def fetch(url):return json.dumps(RIOT).encode() if 'ddragon' in url else page(data)
        result, report=mod.build_catalog({'schemaVersion':1,'entries':[old]}, [],'16.19.1','2026-10-03',fetch)
        self.assertEqual(result['entries'][0]['segments'],old['segments']);self.assertFalse(report['errors'])
        old['descriptionSha256']='changed'
        _, report=mod.build_catalog({'schemaVersion':1,'entries':[old]}, [],'16.19.1','2026-10-03',fetch)
        self.assertTrue(report['errors'])

    def test_ajout_par_lot_idempotent_et_cache_champion(self):
        second='7lS2jAaJG8E';riot=copy.deepcopy(RIOT)
        riot['data']['Ahri']['skins'].append({'id':'103008','name':'Other Ahri'})
        calls=[]
        def fetch(url):
            calls.append(url)
            if 'ddragon' in url:return json.dumps(riot).encode()
            data=metadata()
            if second in url:data['videoDetails'].update(videoId=second,title='Other Ahri'+mod.SUFFIX)
            return page(data)
        candidate={'champion':'Ahri','videoId':second,'minPublishedAt':'2023-02-05'}
        result, report=mod.build_catalog({'schemaVersion':1,'entries':[ENTRY]}, [candidate],'16.19.1','2026-10-03',fetch)
        self.assertFalse(report['errors']);self.assertEqual(len(result['entries']),2)
        self.assertEqual(sum('ddragon' in u for u in calls),1)
        repeated,_=mod.build_catalog(result, [],'16.19.1','2026-10-03',fetch)
        self.assertEqual(result,repeated)

    def test_arrete_les_requetes_apres_limitation_youtube(self):
        calls=[]
        candidates=[{'champion':'Ahri','videoId':f'{number:011}','minPublishedAt':'2025-01-01'} for number in range(20)]
        def limited(url):
            if 'ddragon' in url:return json.dumps(RIOT).encode()
            calls.append(url)
            raise urllib.error.HTTPError(url,429,'Too Many Requests',{},None)
        result,report=mod.build_catalog({'schemaVersion':1,'entries':[ENTRY]},candidates,'16.19.1','2026-10-03',limited)
        self.assertEqual(result['entries'],[ENTRY])
        self.assertLessEqual(len(calls),4)
        self.assertEqual(len(report['errors']),21)
        self.assertTrue(any('429' in error['reason'] for error in report['errors']))

    def test_arrete_aussi_le_lot_si_riot_limite_les_requetes(self):
        calls=[]
        def limited(url):
            calls.append(url)
            raise urllib.error.HTTPError(url,429,'Too Many Requests',{},None)
        candidate={'champion':'Lux','videoId':'7lS2jAaJG8E','minPublishedAt':'2025-01-01'}
        result,report=mod.build_catalog({'schemaVersion':1,'entries':[ENTRY]},[candidate],'16.19.1','2026-10-03',limited)
        self.assertEqual(result['entries'],[ENTRY]);self.assertEqual(len(calls),1)
        self.assertEqual(len(report['errors']),2)

    def test_export_distinct_et_atomique(self):
        with tempfile.TemporaryDirectory() as folder:
            source=Path(folder)/'source.json';output=Path(folder)/'candidate.json'
            source.write_text('original')
            with self.assertRaises(ValueError):mod.write_candidate(source,source,{})
            mod.write_candidate(source,output,{'schemaVersion':1,'entries':[]})
            self.assertEqual(source.read_text(),'original')
            self.assertEqual(json.loads(output.read_text())['entries'],[])


if __name__ == '__main__': unittest.main()
