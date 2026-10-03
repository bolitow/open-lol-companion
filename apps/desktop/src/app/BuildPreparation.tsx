import {useDialogMotion} from '../ui/useDialogMotion';
import {SelectField} from '../ui/SelectField';
import {Disclosure} from '../ui/Disclosure';
import {usePreparation} from './PreparationContext';
import {useEffect,useRef,useState,type ReactNode} from 'react';
import {createPortal} from 'react-dom';
import type {BuildReport,BuildStats,CatalogRecord,DraftSession,Role,RunePage} from '@olc/shared';
import {Icon} from '../ui/Icon';
import {RunePanel,ExpandedRunes,ItemPanel} from './PreparationPanels';
import {ItemImport} from './ItemImportAction';
import {itemSetRequest} from './itemImport';
import {SpellWorkbench} from './SpellWorkbench';
import {RuneWorkbench,type RuneImportContext} from './RuneWorkbench';
import {CatalogButton} from './GameDetails';
import {loadChampionAbilities,type PreparationCatalog} from './catalog';
import {BuildChampionContext} from './BuildChampionContext';
import {buildCopy} from './buildCopy';
import {preparationCopy} from './preparationCopy';
import {buildMetrics,buildRequestKey,runePageFromBuild,skillSequence,variantsFor} from './buildModel';
import {championDetails} from './draft';
import {useBuilds} from './useBuilds';
import type {Locale} from './state';
import './builds.css';

type DisplayProps={readOnly?:boolean;connected?:boolean;importContext?:RuneImportContext;report:BuildReport;records:CatalogRecord[];locale:Locale;onOpen:(record:CatalogRecord)=>void};
const rankNames={fr:['Tous les rangs','Fer','Bronze','Argent','Or','Platine','Émeraude','Diamant','Maître','Grand maître','Challenger'],en:['All ranks','Iron','Bronze','Silver','Gold','Platinum','Emerald','Diamond','Master','Grandmaster','Challenger']};
const rankIds=['ALL','IRON','BRONZE','SILVER','GOLD','PLATINUM','EMERALD','DIAMOND','MASTER','GRANDMASTER','CHALLENGER'];

function Metrics({variant,minGames,locale}:{variant:BuildStats;minGames:number;locale:Locale}){
 const t=buildCopy[locale],m=buildMetrics(variant,minGames),format=new Intl.NumberFormat(locale,{maximumFractionDigits:1});
 return <div className="build-metrics"><span>{new Intl.NumberFormat(locale).format(m.games)} <small>{t.games}</small></span><span title={`${t.frequency} · ${new Intl.NumberFormat(locale).format(m.population)} ${t.games}`}><small>{t.frequency}</small> {m.pickRate===null?'—':`${format.format(m.pickRate)} %`}</span><span><small>{t.winrate}</small> {m.winRate===null?'—':`${format.format(m.winRate)} %`}</span>{m.games<minGames&&<small>{t.sample}</small>}</div>;
}
function VariantPicker({variants,index,onChange,locale,label}:{variants:BuildStats[];index:number;onChange:(i:number)=>void;locale:Locale;label:string}){
 const t=buildCopy[locale];
 return variants.length>1?<SelectField className="build-variant-select" label={`${t.alternative} · ${label}`} value={String(index)} onChange={value=>onChange(Number(value))} options={variants.map((v,i)=>({value:String(i),label:`${t.alternative} ${i+1} · ${new Intl.NumberFormat(locale).format(v.games)} ${t.games}`}))}/>:null;
}
function RecordChoice({id,kind,records,locale,onOpen,children}:{id:string;kind:string;records:CatalogRecord[];locale:Locale;onOpen:(record:CatalogRecord)=>void;children?:ReactNode}){
 const record=records.find(r=>r.kind===kind&&r.id===id);
 return record?<CatalogButton abilityChampion={records.find(candidate=>candidate.kind==='champion'&&candidate.id===record.id.split(':')[0])} record={record} locale={locale} onOpen={onOpen} className="build-record"><span className="build-record-name">{record.name}</span>{children}</CatalogButton>:<span className="build-missing-record" title={`${buildCopy[locale].unknown} · ${id}`}><Icon name="info" size={18}/><small>{buildCopy[locale].unknown}</small>{children}</span>;
}
function CommunityRunes({report,records,locale,onOpen,importContext}:DisplayProps){
 const t=buildCopy[locale],variants=variantsFor(report,'runes'),[index,setIndex]=useState(0),[expanded,setExpanded]=useState(false);
 const variant=variants[index],page=variant?runePageFromBuild(variant.selection,records):null;
 return <section className="surface community-runes"><div className="build-panel-heading"><span>{t.runes}</span><div className="build-rune-actions"><VariantPicker variants={variants} index={index} onChange={setIndex} locale={locale} label={t.runes}/>{page&&!importContext&&<button className="icon-button" aria-label={preparationCopy[locale].expand} onClick={event=>{event.currentTarget.focus();setExpanded(true)}}><Icon name="expand" size={15}/></button>}</div></div>{page&&importContext?<RuneWorkbench key={variant?.selection.join(':')} {...importContext} sourceKey={`${importContext.sourceKey}:${variant?.selection.join(':')}`} page={page} sourceLabel={t.selected} records={records} locale={locale} onOpen={onOpen} statistics={variant&&<Metrics variant={variant} minGames={report.meta.min_games} locale={locale}/>}/>:page?<RunePanel key={variant?.selection.join(':')} records={records} page={page} locale={locale} onOpen={onOpen} sourceLabel={t.selected} returnLabel={t.returnVariant}/>:<p className="build-empty">{variants.length?t.runeMissing:t.categoryMissing}</p>}{variant&&!importContext&&<Metrics variant={variant} minGames={report.meta.min_games} locale={locale}/ >}{expanded&&<ExpandedRunes records={records} page={page} locale={locale} onOpen={onOpen} onClose={()=>setExpanded(false)} sourceLabel={t.selected} returnLabel={t.returnVariant}/>}</section>;
}
function CommunityItems({report,records,locale,onOpen,importContext,connected=false,readOnly=false}:DisplayProps){
 const t=buildCopy[locale],[category,setCategory]=useState<'purchase_order'|'final_items'|'item'|'trinket'>('purchase_order'),[index,setIndex]=useState(0);
 const variants=variantsFor(report,category),variant=variants[index];
 const request=importContext&&variant?itemSetRequest(importContext.championId,importContext.championName,t[category],variant.selection,records):null;
 return <section className="surface build-item-panel"><header className="build-panel-heading"><h2>{t.items}</h2><SelectField value={category} label={t.items} onChange={value=>{setCategory(value as typeof category);setIndex(0)}} options={(['purchase_order','final_items','item','trinket'] as const).map(value=>({value,label:t[value]}))}/></header><VariantPicker variants={variants} index={index} onChange={setIndex} locale={locale} label={t[category]}/><div className="build-item-scroll preparation-scroll">{variant?.selection.length?<ol className={`build-item-sequence ${category==='purchase_order'?'ordered':''}`}>{variant.selection.map((id,i)=><li key={`${id}:${i}`}><RecordChoice id={String(id)} kind="item" records={records} locale={locale} onOpen={onOpen}>{category==='purchase_order'&&<span className="sequence-index">{i+1}</span>}</RecordChoice></li>)}</ol>:<p className="build-empty">{variant?t.emptySelection:t.categoryMissing}</p>}</div>{!readOnly&&<ItemImport request={request} sourceKey={`${importContext?.sourceKey??''}:${category}:${index}`} connected={connected} locale={locale}/>}<p className="build-hint">{category==='purchase_order'?t.purchaseHint:category==='item'?t.itemHint:t.finalHint}</p>{variant&&<Metrics variant={variant} minGames={report.meta.min_games} locale={locale}/>}</section>;
}
function SkillAndSpells({report,records,locale,onOpen,importContext,readOnly=false}:DisplayProps){
 const t=buildCopy[locale],skills=variantsFor(report,'skill_order'),spells=variantsFor(report,'summoner_spells');
 const [skillIndex,setSkillIndex]=useState(0),[spellIndex,setSpellIndex]=useState(0),skill=skills[skillIndex],spell=spells[spellIndex];
 const sequence=skill?skillSequence(skill.selection):null;
 return <section className="surface build-abilities preparation-scroll">
 <div className="build-spells">{readOnly?<><h3>{t.summoner_spells}</h3><VariantPicker variants={spells} index={spellIndex} onChange={setSpellIndex} locale={locale} label={t.summoner_spells}/><div className="build-spell-pair">{spell?.selection.map((id,i)=><RecordChoice key={`${id}:${i}`} id={String(id)} kind="summoner_spell" records={records} locale={locale} onOpen={onOpen}/>)}</div>{spell?<Metrics variant={spell} minGames={report.meta.min_games} locale={locale}/>:<p className="build-empty">{t.categoryMissing}</p>}</>:<SpellWorkbench key={spell?.selection.join(':')??'manual'} draft={importContext?.draft??null} championId={importContext?.championId??report.request.champion_id} sourceKey={`${importContext?.sourceKey??''}:${spell?.selection.join(':')??'manual'}`} source={spell?.selection} variantPicker={<VariantPicker variants={spells} index={spellIndex} onChange={setSpellIndex} locale={locale} label={t.summoner_spells}/>} records={records} locale={locale} onOpen={onOpen} statistics={spell&&<Metrics variant={spell} minGames={report.meta.min_games} locale={locale}/>}/>}</div>
 <Disclosure className="spell-skills-details" label={t.skill_order}><VariantPicker variants={skills} index={skillIndex} onChange={setSkillIndex} locale={locale} label={t.skill_order}/>{sequence?<><div className="build-ability-icons">{(['Q','W','E','R'] as const).map((key,i)=><RecordChoice key={key} id={`${report.request.champion_id}:${key}`} kind="ability" records={records} locale={locale} onOpen={onOpen}><b>{locale==='fr'?['A','Z','E','R'][i]:key}</b></RecordChoice>)}</div><ol className="skill-point-sequence" aria-label={t.sequence}>{sequence.map((key,i)=><li key={i}><small>{i+1}</small><strong className={`skill-${key}`}>{locale==='fr'?{Q:'A',W:'Z',E:'E',R:'R'}[key]:key}</strong></li>)}</ol><p className="build-hint">{t.skillHint}</p>{skill&&<Metrics variant={skill} minGames={report.meta.min_games} locale={locale}/>}</>:<p className="build-empty">{t.categoryMissing}</p>}</Disclosure>
 </section>;
}
function SourceDialog({report,locale,onClose}:{report:BuildReport;locale:Locale;onClose:()=>void}){
 const motion=useDialogMotion(onClose);
 const t=buildCopy[locale],dialog=useRef<HTMLDialogElement>(null),[trigger]=useState(()=>document.activeElement instanceof HTMLElement?document.activeElement:null);
 useEffect(()=>{const node=dialog.current;node?.showModal();return()=>{node?.close();if(trigger?.isConnected)trigger.focus({preventScroll:true})}},[trigger]);
 const date=(value:string)=>Number.isNaN(Date.parse(value))?'—':new Intl.DateTimeFormat(locale,{dateStyle:'medium',timeStyle:'short'}).format(new Date(value));
 return createPortal(<dialog ref={dialog} className="game-detail build-source-dialog motion-surface" data-state={motion.state} inert={motion.closing} onCancel={event=>{event.preventDefault();motion.close()}} aria-label={t.sourceTitle}><div className="game-detail-shell"><header><h2>{t.sourceTitle}</h2><button className="icon-button" aria-label={t.close} onClick={motion.close}><Icon name="close"/></button></header><div className="game-detail-body"><p>{t.sourceHint}</p><dl><dt>{t.scope}</dt><dd>{report.request.patch} · {report.request.platform} · {report.request.queue} · {t.roles[report.request.role]} · {rankNames[locale][rankIds.indexOf(report.request.rank)]??report.request.rank}</dd><dt>{t.collected}</dt><dd>{date(report.meta.source_snapshot_at)}</dd><dt>{t.published}</dt><dd>{date(report.meta.published_at)}</dd><dt>{t.threshold}</dt><dd>{report.meta.min_games} {t.games}</dd></dl><small>{t.sources}</small></div></div></dialog>,document.body);
}
export function CommunityBuildPanels(props:DisplayProps){
 return <div className="community-build-grid"><CommunityRunes {...props}/><CommunityItems {...props}/><SkillAndSpells {...props}/></div>;
}
export function BuildPreparation({draft,catalog,equipped,locale,onOpen,connected=false}:{connected?:boolean;draft:DraftSession|null;catalog:PreparationCatalog;equipped:RunePage|null;locale:Locale;onOpen:(record:CatalogRecord)=>void}){
 const t=buildCopy[locale],local=draft?.allies.find(p=>p.local),[source,setSource]=useState(false);
 const {value:{manual,roleOverride,platform,queue,rank,mode},update}=usePreparation();
 const setManual=(manual:number|null)=>update({manual}),setRoleOverride=(roleOverride:Role|null)=>update({roleOverride}),setPlatform=(platform:string)=>update({platform}),setQueue=(queue:number)=>update({queue}),setRank=(rank:string)=>update({rank}),setMode=(mode:'community'|'equipped')=>update({mode});
 const championId=manual??local?.championId??0,role=roleOverride??(manual===null&&local?.position?local.position.toUpperCase() as Role:'MIDDLE'),champion=championDetails(championId||null,locale);
 const request=championId?{champion_id:championId,patch:catalog.version.split('.').slice(0,2).join('.'),platform,queue,role,rank}:null;
 const {state,retry}=useBuilds(mode==='community'?request:null),report=state?.status==='ready'?state.report:null;
 const [abilities,setAbilities]=useState<{championId:number;locale:Locale;version:string;records:CatalogRecord[]}|null>(null);
 useEffect(()=>{let active=true;if(championId)loadChampionAbilities(locale,championId,catalog.version).then(records=>{if(active)setAbilities({championId,locale,version:catalog.version,records})},()=>{if(active)setAbilities(null)});return()=>{active=false}},[championId,locale,catalog.version]);
 const records=abilities?.championId===championId&&abilities.locale===locale&&abilities.version===catalog.version?[...catalog.records,...abilities.records]:catalog.records;
 // Un dialogue de provenance ou de runes ne doit pas garder le contexte précédent.
 const context=request?buildRequestKey(request):'';
 useEffect(()=>{setSource(false)},[context,mode]);
 const importContext:RuneImportContext={draft,equipped,championId,championName:champion?.name??'',sourceKey:`${context}:${mode}`};
 return <div className="build-workspace">
  <div className="build-toolbar"><BuildChampionContext manual={manual} local={local} locale={locale} onChange={manual=>{setManual(manual);setRoleOverride(null)}}/>
   <div className="build-mode-switch"><button aria-pressed={mode==='community'} onClick={()=>setMode('community')}>{t.community}</button><button aria-pressed={mode==='equipped'} onClick={()=>setMode('equipped')}>{t.equipped} & {t.catalog}</button></div>
   <div className="build-scope" aria-label={t.filters}><label><span>{t.role}</span><SelectField label={t.role} value={role} onChange={value=>setRoleOverride(value as Role)} options={Object.entries(t.roles).map(([value,label])=>({value,label}))}/></label><label><span>{t.region}</span><SelectField label={t.region} value={platform} onChange={setPlatform} options={['EUW1','EUN1','NA1','KR','JP1','BR1','LA1','LA2','OC1','TR1','RU','ME1','SG2','TW2','VN2'].map(value=>({value,label:value}))}/></label><label><span>{t.queue}</span><SelectField label={t.queue} value={String(queue)} onChange={value=>setQueue(Number(value))} options={([420,440,400] as const).map(q=>({value:String(q),label:t.queues[q]}))}/></label><label><span>{t.rank}</span><SelectField label={t.rank} value={rank} onChange={setRank} options={rankIds.map((value,i)=>({value,label:rankNames[locale][i]??value}))}/></label><small className="build-patch">{t.patch} {request?.patch??catalog.version}</small>{mode==='community'&&request&&<button className="icon-button" aria-label={t.refresh} onClick={retry} disabled={state?.status==='loading'}><Icon name="replay" size={15}/></button>}</div>
  </div>
  {mode==='equipped'?<div className="community-build-grid live-preparation"><RuneWorkbench key={`${locale}:${context}`} {...importContext} records={catalog.records} page={equipped} sourceLabel={t.equipped} locale={locale} onOpen={onOpen}/><ItemPanel records={catalog.records} locale={locale} onOpen={onOpen}/><div className="surface build-abilities preparation-scroll"><SpellWorkbench key={`spells:${context}`} draft={draft} championId={championId} sourceKey={context} records={catalog.records} locale={locale} onOpen={onOpen}/></div></div>:report?.builds.length?<CommunityBuildPanels key={`${context}:${report.meta.published_at}:${locale}`} report={report} records={records} locale={locale} onOpen={onOpen} importContext={importContext} connected={connected}/>:<div className="surface build-status" role="status"><Icon name="chart" size={28}/><p>{!request?t.selectFirst:state?.status==='loading'?t.loading:state?.status==='error'?t.errors[state.error]:t.empty}</p>{state?.status==='error'&&!['not_configured','invalid_configuration','desktop_required'].includes(state.error)&&<button className="button" onClick={retry}>{t.retry}</button>}</div>}
  <footer className="build-footer"><small>{t.patch} {catalog.version}{report?` · ${t.independent}`:''}</small>{report&&<button onClick={event=>{event.currentTarget.focus();setSource(true)}}><Icon name="info" size={12}/>{t.source}</button>}</footer>
  {source&&report&&<SourceDialog report={report} locale={locale} onClose={()=>setSource(false)}/ >}
 </div>;
}
