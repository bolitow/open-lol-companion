"""Couverture exhaustive, association exacte et arrêt fournisseur hors réseau."""
import importlib.util
from pathlib import Path
import unittest

SPEC = importlib.util.spec_from_file_location('coverage', Path(__file__).with_name('catalog-skin-spotlights.py'))
mod = importlib.util.module_from_spec(SPEC)
if SPEC.loader is not None and Path(SPEC.origin).exists():
    SPEC.loader.exec_module(mod)

RIOT = {'Ahri': {'data': {'Ahri': {'key': '103', 'name': 'Ahri', 'skins': [
    {'id':'103000','name':'default'}, {'id':'103007','name':'Arcade Ahri'}, {'id':'103008','name':'Star Guardian Ahri'}]}}}}
ENTRY = {'skinId':103007,'championId':103,'videoId':'IPU9_WRcsj4','publishedAt':'2023-02-05','segments':[],'title':'Arcade Ahri Skin Spotlight - League of Legends'}

class SupplementarySearches(unittest.TestCase):
    def test_rapport_csv_reflete_blocage_cible_sans_masquer_la_validation(self):
        report={'coverage':[
            {'skinId':103007,'status':'referenced'},
            {'skinId':103014,'status':'search_incomplete'},
            {'skinId':103002,'status':'needs_review'}], 'summary':{}}
        extra={str(row['skinId']):{'status':'rate_limited'} for row in report['coverage']}
        mod.apply_skin_searches(report,extra)
        self.assertEqual([r['status'] for r in report['coverage']],['referenced','supplier_blocked','needs_review'])
        self.assertTrue(all(r['skinSearchStatus']=='rate_limited' for r in report['coverage']))
        self.assertEqual(report['summary']['statuses']['supplier_blocked'],1)

    def test_cible_uniquement_les_skins_non_resolus_sans_doublon(self):
        rows=[{'skinId':103007,'kind':'skin','name':'Arcade Ahri','status':'referenced'},
              {'skinId':103014,'kind':'skin','name':'Star Guardian Ahri','status':'search_incomplete'},
              {'skinId':103001,'kind':'skin','name':'Dynasty Ahri','status':'outside_date_window'},
              {'skinId':103002,'kind':'skin','name':'Midnight Ahri','status':'needs_review'},
              {'skinId':103008,'kind':'chroma','name':'Popstar Ahri (Amethyst)','status':'chroma_variant'}]
        self.assertEqual([r['skinId'] for r in mod.skin_search_targets(rows+[rows[1]])],[103002,103014])

    def test_recherche_par_identifiant_avec_cache_et_arret_fournisseur(self):
        import tempfile,urllib.error
        with tempfile.TemporaryDirectory() as directory:
            calls=[]
            def fetch(url):
                calls.append(url)
                raise urllib.error.HTTPError(url,429,'rate limit',None,None)
            store=mod.MetadataStore(Path(directory),[],fetch)
            rows=[{'skinId':103014,'kind':'skin','name':'Star Guardian Ahri','status':'search_incomplete'},
                  {'skinId':103002,'kind':'skin','name':'Midnight Ahri','status':'needs_review'}]
            results=mod.supplement_skin_searches(rows,store)
            self.assertEqual(len(calls),1)
            self.assertTrue(all(r['status']=='rate_limited' for r in results.values()))
            self.assertEqual(results['103014']['source'],'https://www.youtube.com/@SkinSpotlights/search?query=Star%20Guardian%20Ahri')


class Coverage(unittest.TestCase):
    def test_recherche_limitee_ne_prouve_pas_absence_video(self):
        self.assertTrue(hasattr(mod, 'coverage_rows'), 'La couverture exhaustive manque')
        rows=mod.coverage_rows(RIOT, [ENTRY], {'Ahri':{'status':'partial'}}, [])
        self.assertEqual(len(rows),2)
        self.assertEqual(rows[0]['status'],'referenced')
        self.assertEqual(rows[1]['status'],'search_incomplete')

    def test_429_separe_de_refus_et_skin_ancien_reste_visible(self):
        self.assertTrue(hasattr(mod, 'coverage_rows'))
        rows=mod.coverage_rows(RIOT, [], {'Ahri':{'status':'rate_limited'}}, [
            {'skinId':103007,'reason':'too_old','videoId':'IPU9_WRcsj4'}])
        self.assertEqual([r['status'] for r in rows], ['outside_date_window','supplier_blocked'])

    def test_correspondance_globale_exacte_pas_selon_recherche_origine(self):
        self.assertTrue(hasattr(mod, 'resolve_title'))
        self.assertEqual(mod.resolve_title('ARCADE Ahri Skin Spotlight - League of Legends',RIOT),('Ahri',103007))
        self.assertIsNone(mod.resolve_title('Arcade Ahri Skin Spotlight - Wild Rift',RIOT))
        self.assertIsNone(mod.resolve_title('Arcade  Ahri Skin Spotlight - League of Legends',RIOT))

    def test_noms_dupliques_non_rapproches(self):
        self.assertTrue(hasattr(mod, 'resolve_title'))
        other={'Other':{'data':{'Other':{'key':'99','name':'Other','skins':[{'id':'99001','name':'Arcade Ahri'}]}}}}
        self.assertIsNone(mod.resolve_title('Arcade Ahri Skin Spotlight - League of Legends',{**RIOT,**other}))

    def test_choix_date_limite_et_plus_recent_deterministe(self):
        self.assertTrue(hasattr(mod, 'choose_entries'))
        other={**ENTRY,'videoId':'7lS2jAaJG8E','publishedAt':'2025-01-01'}
        old={**ENTRY,'skinId':103008,'publishedAt':'2018-01-01'}
        entries,issues=mod.choose_entries([ENTRY,other,old],'2019-10-03')
        self.assertEqual(entries,[other])
        self.assertTrue(any(i['reason']=='too_old' for i in issues))

    def test_arret429_mais_cache_reutilisable(self):
        self.assertTrue(hasattr(mod, 'MetadataStore'))
        import tempfile, urllib.error
        with tempfile.TemporaryDirectory() as directory:
            calls=[]
            def fetch(url):
                calls.append(url)
                raise urllib.error.HTTPError(url,429,'rate limit',None,None)
            store=mod.MetadataStore(Path(directory),[],fetch)
            with self.assertRaises(OSError): store.get('video','IPU9_WRcsj4','https://www.youtube.com/watch?v=IPU9_WRcsj4')
            with self.assertRaises(OSError): store.get('search','Ahri','https://www.youtube.com/@SkinSpotlights/search?query=Ahri')
            self.assertEqual(len(calls),1)
            self.assertEqual(store.stop_reason,'http_429')

class FailureSafety(unittest.TestCase):
    def test_inventaire_incomplet_refuse_candidat_mais_ecrit_rapport(self):
        import json, subprocess, sys, tempfile
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory)
            (root/'catalog.json').write_text(json.dumps({'schemaVersion':1,'entries':[ENTRY]}))
            (root/'sources.json').write_text(json.dumps({'version':'16.19.1','sources':[{'key':'en_US/champion/Ahri.json','provider':'ddragon','version':'16.19.1','url':'https://ddragon.leagueoflegends.com/cdn/16.19.1/data/en_US/champion/Ahri.json'}]}))
            result=subprocess.run([sys.executable,str(Path(__file__).with_name('catalog-skin-spotlights.py')),
                '--catalog',str(root/'catalog.json'),'--sources',str(root/'sources.json'),
                '--cache',str(root/'cache'),'--offline','--minimum-date','2019-10-03',
                '--output',str(root/'candidate.json'),'--report',str(root/'report.json'),
                '--coverage-csv',str(root/'coverage.csv')],capture_output=True,text=True)
            self.assertNotEqual(result.returncode,0,'Inventaire incomplet accepté')
            self.assertFalse((root/'candidate.json').exists())
            self.assertTrue((root/'report.json').exists())
            self.assertNotIn(b'\r\n',(root/'coverage.csv').read_bytes(), 'Le CSV versionné doit utiliser LF')
            self.assertEqual(json.loads((root/'catalog.json').read_text())['entries'],[ENTRY])

    def test_refus_embed_cache_reste_refus_apres429(self):
        import json, tempfile
        from test_update_skin_spotlights import metadata
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory)
            data=metadata();data['playabilityStatus']['playableInEmbed']=False
            (root/'video-IPU9_WRcsj4.json').write_text(json.dumps(data))
            (root/'search-Ahri.json').write_text(json.dumps({'videos':[{'videoId':'IPU9_WRcsj4','title':data['videoDetails']['title']}]}))
            store=mod.MetadataStore(root,[],offline=True);store.stop_reason='http_429'
            result,report=mod.collect(RIOT,{'schemaVersion':1,'entries':[]},store,'2019-10-03','2026-10-03','16.19.1')
            issue=next(i for i in report['issues'] if i.get('skinId')==103007)
            self.assertEqual(issue['reason'],'verification_failed')
            self.assertEqual(report['coverage'][0]['status'],'needs_review')
            self.assertEqual(result['entries'],[])

class Reporting(unittest.TestCase):
    def test_refus_cache_prime_sur_recherche_bloquee(self):
        rows=mod.coverage_rows(RIOT,[],{'Ahri':{'status':'rate_limited'}},[
            {'skinId':103007,'videoId':'IPU9_WRcsj4','reason':'verification_failed'}])
        self.assertEqual(rows[0]['status'],'needs_review')
        self.assertEqual(rows[1]['status'],'supplier_blocked')

    def test_voix_et_pbe_ne_polluent_pas_liste_revue_skins(self):
        import json,tempfile
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory)
            (root/'search-Ahri.json').write_text(json.dumps({'videos':[
                {'videoId':'IPU9_WRcsj4','title':'Ahri Voice - English'},
                {'videoId':'7lS2jAaJG8E','title':'Star Guardian Ahri Skin Spotlight - Pre-Release - PBE Preview - League of Legends'}]}))
            store=mod.MetadataStore(root,[],offline=True)
            _,report=mod.collect(RIOT,{'schemaVersion':1,'entries':[]},store,'2019-10-03','2026-10-03','16.19.1')
            self.assertEqual(report['issues'],[])
            self.assertEqual(report['summary']['discardedNonSkinTitles'],2)

class NetworkSetup(unittest.TestCase):
    def test_erreur_certificat_suspend_les_autres_requetes(self):
        import ssl,tempfile,urllib.error
        with tempfile.TemporaryDirectory() as directory:
            calls=[]
            def fetch(url):
                calls.append(url)
                raise urllib.error.URLError(ssl.SSLCertVerificationError('missing CA'))
            store=mod.MetadataStore(Path(directory),[],fetch)
            for video in ('IPU9_WRcsj4','7lS2jAaJG8E'):
                with self.assertRaises(OSError):
                    store.get('video',video,'https://www.youtube.com/watch?v='+video)
            self.assertEqual(len(calls),1)
            self.assertEqual(store.stop_reason,'tls_certificate_error')

class ChromaInventory(unittest.TestCase):
    def test_chroma_identifie_par_riot_n_est_pas_un_skin_sans_video(self):
        import copy
        riot=copy.deepcopy(RIOT)
        riot['Ahri']['data']['Ahri']['skins'].append({'id':'103009','name':'Arcade Ahri (Ruby)','parentSkin':7})
        rows=mod.coverage_rows(riot,[ENTRY],{'Ahri':{'status':'partial'}},[])
        self.assertEqual(rows[-1]['status'],'chroma_variant')
        self.assertEqual(rows[-1]['kind'],'chroma')
        self.assertIsNone(mod.resolve_title('Arcade Ahri (Ruby) Skin Spotlight - League of Legends',riot))
        self.assertEqual(rows[0]['kind'],'skin')
