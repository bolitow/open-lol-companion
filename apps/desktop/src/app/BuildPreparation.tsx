import {BuildSummaryView,SkillTimings,ItemTimings,BuildOmissions,hasBuildObservations} from './BuildObservations';
import {BuildPopulation,populationName} from './BuildPopulation';
import {RankSelect} from './RankSelect';
import {rankLabel} from './buildRanks';
import {preparationRequest} from './buildContext';
import {initialState} from './state';
import {usePreparation} from './PreparationContext';
import {useEffect,useRef,useState,type ReactNode} from 'react';
import {createPortal} from 'react-dom';
import type {BuildReport,BuildStats,CatalogRecord,DraftSession,Role,RunePage} from '@olc/shared';
import {Icon} from '../ui/Icon';
import {RunePanel,ExpandedRunes,ItemPanel} from './PreparationPanels';
import {ItemImport} from './ItemImportAction';
import {isImportableCategory,itemAdjustments,itemSetPlan} from './itemImport';
import {SpellWorkbench} from './SpellWorkbench';
import {RuneWorkbench,type RuneImportContext} from './RuneWorkbench';
import {CatalogButton} from './GameDetails';
import {loadChampionAbilities,type PreparationCatalog} from './catalog';
import championIndex from '../../public/game-data/champions.json';
import {buildCopy} from './buildCopy';
import {preparationCopy} from './preparationCopy';
import {buildMetrics,buildRequestKey,runePageFromBuild,skillSequence,variantsFor} from './buildModel';
import {championDetails} from './draft';
import {useBuilds} from './useBuilds';
import type {Locale} from './state';
import './builds.css';

type DisplayProps={readOnly?:boolean;connected?:boolean;importContext?:RuneImportContext;report:BuildReport;records:CatalogRecord[];locale:Locale;onOpen:(record:CatalogRecord)=>void};

function Metrics({variant,minGames,locale}:{variant:BuildStats;minGames:number;locale:Locale}){
 const t=buildCopy[locale],m=buildMetrics(variant,minGames),format=new Intl.NumberFormat(locale,{maximumFractionDigits:1});
 return <div className="build-metrics"><span>{new Intl.NumberFormat(locale).format(m.games)} <small>{t.games}</small></span><span title={`${t.frequency} · ${new Intl.NumberFormat(locale).format(m.population)} ${t.games}`}><small>{t.frequency}</small> {m.pickRate===null?'—':`${format.format(m.pickRate)} %`}</span><span><small>{t.winrate}</small> {m.winRate===null?'—':`${format.format(m.winRate)} %`}</span>{m.games<minGames&&<small>{t.sample}</small>}</div>;
}
function VariantPicker({variants,index,onChange,locale,label}:{variants:BuildStats[];index:number;onChange:(i:number)=>void;locale:Locale;label:string}){
 const t=buildCopy[locale];
 return variants.length>1?<select className="build-variant-select" aria-label={`${t.alternative} · ${label}`} value={index} onChange={event=>onChange(Number(event.target.value))}>{variants.map((v,i)=><option value={i} key={v.selection.join(':')}>{t.alternative} {i+1} · {new Intl.NumberFormat(locale).format(v.games)} {t.games}</option>)}</select>:null;
}
function RecordChoice({id,kind,records,locale,onOpen,children}:{id:string;kind:string;records:CatalogRecord[];locale:Locale;onOpen:(record:CatalogRecord)=>void;children?:ReactNode}){
 const record=records.find(r=>r.kind===kind&&r.id===id);
 return record?<CatalogButton record={record} locale={locale} onOpen={onOpen} className="build-record"><span className="build-record-name">{record.name}</span>{children}</CatalogButton>:<span className="build-missing-record" title={`${buildCopy[locale].unknown} · ${id}`}><Icon name="info" size={18}/><small>{buildCopy[locale].unknown}</small>{children}</span>;
}
function CommunityRunes({report,records,locale,onOpen,importContext}:DisplayProps){
 const t=buildCopy[locale],variants=variantsFor(report,'runes'),[index,setIndex]=useState(0),[expanded,setExpanded]=useState(false);
 const variant=variants[index],page=variant?runePageFromBuild(variant.selection,records):null;
 return <section className="surface community-runes"><div className="build-panel-heading"><span>{t.runes}</span><div className="build-rune-actions"><VariantPicker variants={variants} index={index} onChange={setIndex} locale={locale} label={t.runes}/>{page&&!importContext&&<button className="icon-button" aria-label={preparationCopy[locale].expand} onClick={event=>{event.currentTarget.focus();setExpanded(true)}}><Icon name="expand" size={15}/></button>}</div></div>{page&&importContext?<RuneWorkbench key={variant?.selection.join(':')} {...importContext} sourceKey={`${importContext.sourceKey}:${variant?.selection.join(':')}`} page={page} sourceLabel={t.selected} records={records} locale={locale} onOpen={onOpen} statistics={variant&&<Metrics variant={variant} minGames={report.meta.min_games} locale={locale}/>}/>:page?<RunePanel key={variant?.selection.join(':')} records={records} page={page} locale={locale} onOpen={onOpen} sourceLabel={t.selected} returnLabel={t.returnVariant}/>:<p className="build-empty">{variants.length?t.runeMissing:t.categoryMissing}</p>}{variant&&!importContext&&<Metrics variant={variant} minGames={report.meta.min_games} locale={locale}/ >}{expanded&&<ExpandedRunes records={records} page={page} locale={locale} onOpen={onOpen} onClose={()=>setExpanded(false)} sourceLabel={t.selected} returnLabel={t.returnVariant}/>}</section>;
}
function CommunityItems({report,records,locale,onOpen,importContext,connected=false,readOnly=false}:DisplayProps){
 const t=buildCopy[locale],[category,setCategory]=useState<'purchase_order'|'final_items'|'item'|'trinket'>('purchase_order'),[index,setIndex]=useState(0);
 const variants=variantsFor(report,category),variant=variants[index];
 const importable=isImportableCategory(category),plan=importable&&importContext&&variant?itemSetPlan(importContext.championId,importContext.championName,t[category],variant.selection,records):null;
 return <section className="surface build-item-panel"><header className="build-panel-heading"><h2>{t.items}</h2><select value={category} aria-label={t.items} onChange={e=>{setCategory(e.target.value as typeof category);setIndex(0)}}>{(['purchase_order','final_items','item','trinket'] as const).map(c=><option key={c} value={c}>{t[c]}</option>)}</select></header><VariantPicker variants={variants} index={index} onChange={setIndex} locale={locale} label={t[category]}/><div className="build-item-scroll preparation-scroll">{variant?.selection.length?<ol className={`build-item-sequence ${category==='purchase_order'?'ordered':''}`}>{variant.selection.map((id,i)=><li key={`${id}:${i}`}><RecordChoice id={String(id)} kind="item" records={records} locale={locale} onOpen={onOpen}>{category==='purchase_order'&&<span className="sequence-index">{i+1}</span>}</RecordChoice></li>)}</ol>:<p className="build-empty">{variant?t.emptySelection:t.categoryMissing}</p>}</div>{!readOnly&&<ItemImport request={plan?.request??null} adjustments={plan?itemAdjustments(plan):null} importable={importable} sourceKey={`${importContext?.sourceKey??''}:${category}:${index}`} connected={connected} locale={locale}/>}<p className="build-hint">{category==='purchase_order'?t.purchaseHint:category==='item'?t.itemHint:t.finalHint}</p>{variant&&<Metrics variant={variant} minGames={report.meta.min_games} locale={locale}/>}<ItemTimings report={report} records={records} locale={locale}/></section>;
}
function SkillAndSpells({report,records,locale,onOpen,importContext,readOnly=false}:DisplayProps){
 const t=buildCopy[locale],skills=variantsFor(report,'skill_order'),spells=variantsFor(report,'summoner_spells');
 const [skillIndex,setSkillIndex]=useState(0),[spellIndex,setSpellIndex]=useState(0),skill=skills[skillIndex],spell=spells[spellIndex];
 const sequence=skill?skillSequence(skill.selection):null;
 return <section className="surface build-abilities preparation-scroll">
 <div className="build-spells">{readOnly?<><h3>{t.summoner_spells}</h3><VariantPicker variants={spells} index={spellIndex} onChange={setSpellIndex} locale={locale} label={t.summoner_spells}/><div className="build-spell-pair">{spell?.selection.map((id,i)=><RecordChoice key={`${id}:${i}`} id={String(id)} kind="summoner_spell" records={records} locale={locale} onOpen={onOpen}/>)}</div>{spell?<Metrics variant={spell} minGames={report.meta.min_games} locale={locale}/>:<p className="build-empty">{t.categoryMissing}</p>}</>:<SpellWorkbench key={spell?.selection.join(':')??'manual'} draft={importContext?.draft??null} championId={importContext?.championId??report.request.champion_id} sourceKey={`${importContext?.sourceKey??''}:${spell?.selection.join(':')??'manual'}`} source={spell?.selection} variantPicker={<VariantPicker variants={spells} index={spellIndex} onChange={setSpellIndex} locale={locale} label={t.summoner_spells}/>} records={records} locale={locale} onOpen={onOpen} statistics={spell&&<Metrics variant={spell} minGames={report.meta.min_games} locale={locale}/>}/>}</div>
 <details className="spell-skills-details"><summary>{t.skill_order}</summary><VariantPicker variants={skills} index={skillIndex} onChange={setSkillIndex} locale={locale} label={t.skill_order}/>{sequence?<><div className="build-ability-icons">{(['Q','W','E','R'] as const).map((key,i)=><RecordChoice key={key} id={`${report.request.champion_id}:${key}`} kind="ability" records={records} locale={locale} onOpen={onOpen}><b>{locale==='fr'?['A','Z','E','R'][i]:key}</b></RecordChoice>)}</div><ol className="skill-point-sequence" aria-label={t.sequence}>{sequence.map((key,i)=><li key={i}><small>{i+1}</small><strong className={`skill-${key}`}>{locale==='fr'?{Q:'A',W:'Z',E:'E',R:'R'}[key]:key}</strong></li>)}</ol><p className="build-hint">{t.skillHint}</p>{skill&&<Metrics variant={skill} minGames={report.meta.min_games} locale={locale}/>}</>:<p className="build-empty">{t.categoryMissing}</p>}</details>
 <SkillTimings report={report} locale={locale}/>
 </section>;
}
function SourceDialog({report,locale,onClose}:{report:BuildReport;locale:Locale;onClose:()=>void}){
 const t=buildCopy[locale],dialog=useRef<HTMLDialogElement>(null),[trigger]=useState(()=>document.activeElement instanceof HTMLElement?document.activeElement:null);
 useEffect(()=>{const node=dialog.current;node?.showModal();return()=>{node?.close();if(trigger?.isConnected)trigger.focus({preventScroll:true})}},[trigger]);
 const date=(value:string)=>Number.isNaN(Date.parse(value))?'—':new Intl.DateTimeFormat(locale,{dateStyle:'medium',timeStyle:'short'}).format(new Date(value));
 return createPortal(<dialog ref={dialog} className="game-detail build-source-dialog" onCancel={event=>{event.preventDefault();onClose()}} aria-label={t.sourceTitle}><div className="game-detail-shell"><header><h2>{t.sourceTitle}</h2><button className="icon-button" aria-label={t.close} onClick={onClose}><Icon name="close"/></button></header><div className="game-detail-body"><p>{t.sourceHint}</p><dl><dt>{t.scope}</dt><dd>{report.request.patch} · {report.request.platform} · {report.request.queue} · {t.roles[report.request.role]} · {rankLabel(report.request.rank,locale)}</dd><dt>{t.collected}</dt><dd>{date(report.meta.source_snapshot_at)}</dd><dt>{t.published}</dt><dd>{date(report.meta.published_at)}</dd><dt>{t.threshold}</dt><dd>{report.meta.min_games} {t.games}</dd></dl><BuildSummaryView report={report} locale={locale}/><BuildOmissions report={report} locale={locale}/><BuildPopulation report={report} locale={locale}/><small>{t.sources}</small></div></div></dialog>,document.body);
}
export function CommunityBuildPanels(props:DisplayProps){
 return <div className="community-build-grid"><CommunityRunes {...props}/><CommunityItems {...props}/><SkillAndSpells {...props}/></div>;
}
export function BuildPreparation({draft,catalog,equipped,locale,onOpen,connected=false}:{connected?:boolean;draft:DraftSession|null;catalog:PreparationCatalog;equipped:RunePage|null;locale:Locale;onOpen:(record:CatalogRecord)=>void}){
 const t=buildCopy[locale],local=draft?.allies.find(p=>p.local),[source,setSource]=useState(false);
 const {value,session,update}=usePreparation();
 const {manual,roleOverride,platform,queue,rank,mode,matchup}=value;
 const setManual=(manual:number|null)=>update({manual,manualCell:null}),setRoleOverride=(roleOverride:Role|null)=>update({roleOverride}),setPlatform=(platform:string)=>update({platform}),setQueue=(queue:number)=>update({queue}),setRank=(rank:string)=>update({rank}),setMode=(mode:'community'|'equipped')=>update({mode});
 const request=preparationRequest(session??{...initialState.session,draft},value,catalog.version);
 const championId=manual??local?.championId??0,role=request?.role??roleOverride??'UNKNOWN',champion=championDetails(championId||null,locale);
 const {state,retry}=useBuilds(mode==='community'?request:null),report=state?.status==='ready'?state.report:null;
 const champions=Object.entries(championIndex).map(([id,entry])=>({id,name:entry[locale]})).sort((a,b)=>a.name.localeCompare(b.name,locale));
 const [abilities,setAbilities]=useState<{championId:number;locale:Locale;version:string;records:CatalogRecord[]}|null>(null);
 useEffect(()=>{let active=true;if(championId)loadChampionAbilities(locale,championId,catalog.version).then(records=>{if(active)setAbilities({championId,locale,version:catalog.version,records})},()=>{if(active)setAbilities(null)});return()=>{active=false}},[championId,locale,catalog.version]);
 const records=abilities?.championId===championId&&abilities.locale===locale&&abilities.version===catalog.version?[...catalog.records,...abilities.records]:catalog.records;
 // Un dialogue de provenance ou de runes ne doit pas garder le contexte précédent.
 const context=request?buildRequestKey(request):'';
 useEffect(()=>{setSource(false)},[context,mode]);
 const importContext:RuneImportContext={draft,equipped,championId,championName:champion?.name??'',sourceKey:`${context}:${mode}`};
 return <div className="build-workspace">
  <div className="build-toolbar"><div className={`build-context ${manual===null?'following':'browsing'}`}>{champion&&<img src={champion.image} alt=""/>}<label><small>{manual===null&&local?.championId?t.following:t.manual}</small><select aria-label={t.champion} value={championId} onChange={e=>{setManual(Number(e.target.value)||null);setRoleOverride(null)}}><option value={0}>{t.choose}</option>{champions.map(c=><option key={c.id} value={c.id}>{c.name}</option>)}</select></label>{manual!==null&&local?.championId&&<button className="return-pick" aria-label={t.follow} onClick={()=>{setManual(null);setRoleOverride(null)}}><Icon name="back" size={17}/><span>{local?.locked?buildCopy[locale].lockedPick:buildCopy[locale].prepick} · {championDetails(local?.championId??null,locale)?.name}</span></button>}</div>
   <div className="build-mode-switch"><button aria-pressed={mode==='community'} onClick={()=>setMode('community')}>{t.community}</button><button aria-pressed={mode==='equipped'} onClick={()=>setMode('equipped')}>{t.equipped} & {t.catalog}</button></div>
   <div className="build-scope" aria-label={t.filters}><label><span>{t.role}</span><select aria-label={t.role} value={role} onChange={e=>setRoleOverride(e.target.value as Role)}>{Object.entries(t.roles).map(([value,label])=><option key={value} value={value}>{label}</option>)}</select></label><label><span>{t.region}</span><select aria-label={t.region} value={platform} onChange={e=>setPlatform(e.target.value)}>{['EUW1','EUN1','NA1','KR','JP1','BR1','LA1','LA2','OC1','TR1','RU','ME1','SG2','TW2','VN2'].map(r=><option key={r}>{r}</option>)}</select></label><label><span>{t.queue}</span><select aria-label={t.queue} value={queue} onChange={e=>setQueue(Number(e.target.value))}>{![420,440,400].includes(queue)&&<option value={queue}>{queue}</option>}{([420,440,400] as const).map(q=><option key={q} value={q}>{t.queues[q]}</option>)}</select></label><label><span>{t.rank}</span><RankSelect queue={queue} rank={rank} locale={locale} onChange={setRank} allLabel={report?.request.rank==='ALL'?populationName(report.meta.population_label,'ALL',locale):undefined}/></label><small className="build-patch">{t.patch} {request?.patch??catalog.version}</small>{mode==='community'&&request&&<button className="icon-button" aria-label={t.refresh} onClick={retry} disabled={state?.status==='loading'}><Icon name="replay" size={15}/></button>}</div>
  {matchup&&<div className="build-matchup" role="status"><span>{t.matchupChosen} · {championDetails(matchup.championId,locale)?.name??t.unknown}</span><small>{t.matchupUnavailable}</small><button className="icon-button" aria-label={t.clearMatchup} onClick={()=>update({matchup:null})}><Icon name="close" size={14}/></button></div>}
  {report&&<BuildPopulation report={report} locale={locale}/>}
  </div>
  {mode==='equipped'?<div className="community-build-grid live-preparation"><RuneWorkbench key={`${locale}:${context}`} {...importContext} records={catalog.records} page={equipped} sourceLabel={t.equipped} locale={locale} onOpen={onOpen}/><ItemPanel records={catalog.records} locale={locale} onOpen={onOpen}/><div className="surface build-abilities preparation-scroll"><SpellWorkbench key={`spells:${context}`} draft={draft} championId={championId} sourceKey={context} records={catalog.records} locale={locale} onOpen={onOpen}/></div></div>:report&&hasBuildObservations(report)?<CommunityBuildPanels key={`${context}:${report.meta.published_at}:${locale}`} report={report} records={records} locale={locale} onOpen={onOpen} importContext={importContext} connected={connected}/>:<div className="surface build-status" role="status"><Icon name="chart" size={28}/><p>{!request?(championId?t.chooseRole:t.selectFirst):state?.status==='loading'?t.loading:state?.status==='error'?t.errors[state.error]:t.empty}</p>{state?.status==='error'&&!['not_configured','invalid_configuration','desktop_required'].includes(state.error)&&<button className="button" onClick={retry}>{t.retry}</button>}</div>}
  <footer className="build-footer"><small>{t.patch} {catalog.version}{report?` · ${t.independent}`:''}</small>{report&&<button onClick={event=>{event.currentTarget.focus();setSource(true)}}><Icon name="info" size={12}/>{t.source}</button>}</footer>
  {source&&report&&<SourceDialog report={report} locale={locale} onClose={()=>setSource(false)}/ >}
 </div>;
}
