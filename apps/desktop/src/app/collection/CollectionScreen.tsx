import {usePresence} from '../../ui/usePresence';
import '../../ui/detailLayout.css';
import skinLines from './skinLines.json';
import {useLayoutEffect, useMemo, useRef} from 'react';
import type {CollectionSkin, SkinRarity} from '../../../../../packages/shared/src/collection';
import type {Locale} from '../state';
import {championDirectory} from '../championDirectory';
import {Icon} from '../../ui/Icon';
import {SelectField} from '../../ui/SelectField';
import type {CollectionHookResult} from './useCollection';
import {canEditWishes, collectionAccountKey, collectionSummary, filterCollection, collectionVisibleCount, nextCollectionCount, type CollectionFilter, type CollectionView} from './collectionModel';
import {collectionCopy} from './collectionCopy';
import './collection.css';
import {SkinSpotlight} from './SkinSpotlight';
import {SkinImage} from './SkinImage';
import {useSkinImagePreloader} from './skinImagePreload';

export interface CollectionScreenProps {locale: Locale; collection: CollectionHookResult; view: CollectionView; update: (patch: Partial<CollectionView>) => void}
function Ownership({skin, locale}: {skin: CollectionSkin; locale: Locale}) {
    return <span title={collectionCopy[locale].ownership[skin.ownership]} className={`collection-ownership is-${skin.ownership}`}><Icon name={skin.ownership === 'owned' ? 'check' : skin.ownership === 'unknown' ? 'info' : skin.ownership === 'temporary' ? 'clock' : 'circle'} size={14}/><span className="collection-ownership-label">{collectionCopy[locale].ownership[skin.ownership]}</span></span>;
}
export function CollectionScreen({locale, collection, view, update}: CollectionScreenProps) {
    const {state, pending, error, refresh} = collection, t = collectionCopy[locale], number = new Intl.NumberFormat(locale);
    const scroll = useRef<HTMLDivElement>(null), key = collectionAccountKey(state), previousAccount = useRef(key);
    const sameAccount = previousAccount.current === key;
    const previews = useSkinImagePreloader(key);
    const cards = useMemo(() => filterCollection(state, view, locale), [state.skins, state.wishes, view.query, view.filter, view.championId, view.rarity, view.seriesId, locale]);
    const visibleCount = collectionVisibleCount(view.visibleCount, cards.length);
    const showMore = () => update({visibleCount: nextCollectionCount(visibleCount, cards.length)});
    const summary = useMemo(() => collectionSummary(state), [state.skins]);
    const wished = useMemo(() => new Set(state.wishes), [state.wishes]);
    const selected = sameAccount ? state.skins.find(skin => skin.id === view.selectedId) : undefined;
    const champions = useMemo(() => {
        const ids = new Set(state.skins.map(skin => skin.champion_id));
        return championDirectory.champions.filter(champion => ids.has(champion.id)).sort((a, b) => a.names[locale].localeCompare(b.names[locale], locale));
    }, [state.skins, locale]);
    const series=useMemo(()=>{const ids=new Set(state.skins.flatMap(skin=>skin.series_ids));return skinLines.entries.filter(line=>ids.has(line.id)).sort((a,b)=>a.names[locale].localeCompare(b.names[locale],locale));},[state.skins,locale]);
    const rarities=useMemo(()=>new Set(state.skins.map(skin=>skin.rarity).filter((rarity):rarity is SkinRarity=>rarity!=null)),[state.skins]);
    useLayoutEffect(() => {if (scroll.current) scroll.current.scrollTop = view.scrollTop;}, []);
    useLayoutEffect(() => {if (view.scrollTop === 0 && scroll.current) scroll.current.scrollTop = 0;}, [view.scrollTop]);
    useLayoutEffect(() => {
        if (previousAccount.current !== key) {previousAccount.current = key; update({selectedId: null, scrollTop: 0});}
    }, [key, update]);
    const close = () => {const id = view.selectedId; update({selectedId: null}); requestAnimationFrame(() => scroll.current?.querySelector<HTMLButtonElement>(`[data-skin="${id}"]`)?.focus({preventScroll: true}));};
    const reset = () => update({query: '', championId: null, rarity:null, seriesId:null, filter: 'all', scrollTop: 0, visibleCount: 60});
    return <div className={`collection-screen ${selected ? 'has-selection' : ''}`}>
        {(state.stale || state.storage_error || error) && <div className="collection-notice" role={error || state.storage_error ? 'alert' : 'status'}>{error ? t.errors[error] : state.storage_error ? t.storageError : state.status === 'disconnected' ? t.cached : t.stale}</div>}
        <div className="collection-workspace detail-layout" data-expanded={Boolean(selected)}>
            <section className="collection-library" aria-label={t.title} aria-busy={state.status === 'loading'}>
                <div className="collection-controls"><div className="collection-filters"><label className="collection-search"><Icon name="search" size={16}/><input autoCorrect="off" autoComplete="off" autoCapitalize="off" spellCheck={false} aria-label={t.search} placeholder={t.search} value={view.query} onChange={event => update({query: event.target.value, scrollTop: 0})}/></label>
                    <SelectField label={t.champion} value={view.championId === null ? 'all' : String(view.championId)} onChange={value => update({championId: value === 'all' ? null : Number(value), scrollTop: 0})} options={[{value: 'all', label: t.allChampions}, ...champions.map(champion => ({value: String(champion.id), label: champion.names[locale]}))]}/>
                    <SelectField label={t.rarity} value={view.rarity??'all'} onChange={value=>update({rarity:value==='all'?null:value as SkinRarity})} options={[{value:'all',label:t.allRarities},...Object.entries(t.rarities).filter(([key])=>rarities.has(key as SkinRarity)).map(([value,label])=>({value,label}))]}/>
                    <SelectField label={t.series} value={view.seriesId===null?'all':String(view.seriesId)} onChange={value=>update({seriesId:value==='all'?null:Number(value)})} options={[{value:'all',label:t.allSeries},...series.map(line=>({value:String(line.id),label:line.names[locale]}))]}/>
                </div>
                <div className="collection-summary-row"><div className="collection-filter-tabs" role="group" aria-label={t.title}>{(Object.keys(t.filters) as CollectionFilter[]).map(filter => <button key={filter} aria-pressed={view.filter === filter} onClick={() => update({filter, scrollTop: 0})}>{t.filters[filter]}</button>)}</div>
                <div className="collection-toolbar"><div className="collection-counts" role="status" aria-label={t.count}>
                    {cards.length !== summary.total && <strong>{number.format(cards.length)} {t.results}</strong>}
                    <span>{number.format(summary.owned)} {t.owned} / {number.format(summary.total)} {t.skins}</span>
                    {summary.temporary > 0 && <small>{number.format(summary.temporary)} {t.temporary}</small>}
                    {summary.unknown > 0 && <small>{number.format(summary.unknown)} {t.unknown}</small>}
                </div><button className="icon-button" aria-label={t.refresh} title={t.refresh} disabled={pending || state.status === 'loading'} onClick={() => void refresh()}><Icon name="replay" size={15}/></button></div></div></div>
                <div className="collection-scroll" ref={scroll} onScroll={event => {const element = event.currentTarget; update({scrollTop: element.scrollTop, ...(element.scrollHeight - element.scrollTop - element.clientHeight < 320 && visibleCount < cards.length ? {visibleCount: nextCollectionCount(visibleCount, cards.length)} : {})});}}>
                    <div className="collection-grid">{cards.slice(0, visibleCount).map((skin,index) => <button key={skin.id} data-skin={skin.id} onPointerEnter={()=>previews.warm(skin.splash_url)} onPointerLeave={previews.cancelIntent} onFocus={()=>previews.warm(skin.splash_url)} onBlur={previews.cancelIntent} className={`collection-card ${selected?.id === skin.id ? 'selected' : ''}`} aria-pressed={selected?.id === skin.id} onClick={() => update({selectedId: skin.id})}>
                        <span className="collection-card-art"><SkinImage key={skin.tile_url} url={skin.tile_url} label={t.imageUnavailable} priority={index<8}/>{wished.has(skin.id) && <span className="collection-wished" aria-label={t.wish}><Icon name="pin" size={14}/></span>}</span>
                        <span className="collection-card-caption"><strong title={skin.name}>{skin.name}</strong><Ownership skin={skin} locale={locale}/></span></button>)}</div>
                    {visibleCount < cards.length && <div className="collection-more"><span>{number.format(visibleCount)} / {number.format(cards.length)}</span><button className="button" onClick={showMore}>{t.showMore}</button></div>}
                    {!cards.length && <div className="collection-empty" role="status"><Icon name="sparkles" size={28}/><p>{state.skins.length ? t.empty : t.status[state.status]}</p>{state.skins.length > 0 && <button className="button" onClick={reset}>{t.reset}</button>}</div>}
                </div>
            </section>
            <div className="detail-slot"><CollectionDetail key={key} skin={selected ?? null} locale={locale} collection={collection} wished={wished} close={close}/></div>
        </div>
    </div>;
}

/** Une nouvelle identité remonte ce composant : aucune ancienne fiche retenue entre comptes. */
function CollectionDetail({skin,locale,collection,wished,close}:{skin:CollectionSkin|null;locale:Locale;collection:CollectionHookResult;wished:Set<number>;close:()=>void}){
    const presence=usePresence(skin,240), selected=presence.value;
    const {state,pending,setWish}=collection,t=collectionCopy[locale];
    if(!selected)return null;
    return <aside className="collection-detail motion-panel" data-state={presence.closing?'closed':'open'} inert={presence.closing} aria-hidden={presence.closing} aria-label={selected.name} onKeyDown={event=>{if(event.key==='Escape'){event.stopPropagation();close()}}}>
                <div key={selected.id} className="collection-detail-content"><div className="collection-detail-art"><SkinImage key={selected.splash_url} url={selected.splash_url ?? selected.tile_url} placeholderUrl={selected.tile_url} priority label={t.imageUnavailable}/>
                    <button className="icon-button" aria-label={t.close} title={t.close} onClick={close}><Icon name="close" size={18}/></button>
                    <div className="collection-detail-heading"><h2>{selected.name}</h2>
                        <div className="collection-detail-meta"><Ownership skin={selected} locale={locale}/><div className="collection-skin-tags">{selected.rarity&&<span>{t.rarities[selected.rarity]}</span>}{skinLines.entries.filter(line=>selected.series_ids.includes(line.id)).map(line=><span key={line.id}>{line.names[locale]}</span>)}</div></div>
                    </div>
                    <button className={`collection-wish-action ${wished.has(selected.id) ? 'primary' : ''}`} disabled={!canEditWishes(state) || pending} aria-label={wished.has(selected.id) ? t.removeWish : t.addWish} title={wished.has(selected.id) ? t.removeWish : t.addWish} aria-pressed={wished.has(selected.id)} onClick={() => void setWish(selected.id, !wished.has(selected.id))}><Icon name="pin" size={16}/></button>
                </div>
                <div className="collection-detail-body">
                    <SkinSpotlight enabled={!presence.closing} key={selected.id} skinId={selected.id} championId={selected.champion_id} locale={locale}/>
                </div>
    </div></aside>;
}
