import {usePreparation} from '../PreparationContext';
import {useEffect, useState} from 'react';
import type {CatalogRecord, LiveSession} from '@olc/shared';
import type {Locale} from '../state';
import {GameDetails} from '../GameDetails';
import {buildCopy} from '../buildCopy';
import {buildRequestKey} from '../buildModel';
import {useBuilds} from '../useBuilds';
import {LiveBuilds} from './LiveBuilds';
import {LiveSummary} from './LiveSummary';
import {connectLiveCatalog, type LiveCatalogState} from './liveCatalog';
import {liveBuildContext, liveBuildRequest} from './liveModel';
import {liveCopy} from './liveCopy';
import {useLiveSession} from './useLiveSession';
import './live.css';

function LiveBuildWorkspace({session, locale, championId}: {session: LiveSession; locale: Locale; championId: number}) {
  const [catalogState, setCatalogState] = useState<LiveCatalogState>({catalog: null, abilities: [], error: false});
  const [attempt, setAttempt] = useState(0);
  const [detail, setDetail] = useState<CatalogRecord | null>(null);
  useEffect(() => connectLiveCatalog(locale, championId, setCatalogState), [locale, championId, attempt]);
  const catalog = catalogState.catalog;
  const {value:preparation,defaultRank='EMERALD_PLUS'}=usePreparation();
  const request = catalog ? liveBuildRequest(session, catalog.version,preparation.rankOverride??defaultRank) : null;
  const {state, retry} = useBuilds(request);
  const records = catalog ? [...catalog.records, ...catalogState.abilities] : [];
  const t = buildCopy[locale];
  if (!catalog) return <div className="surface live-build-status" role="status">
    <p>{catalogState.error ? liveCopy[locale].catalogError : liveCopy[locale].catalogLoading}</p>
    {catalogState.error && <button className="button" onClick={() => { setCatalogState({catalog: null, abilities: [], error: false}); setAttempt(value => value + 1); }}>{t.retry}</button>}
  </div>;
  return <>
    {state?.status === 'ready' ? <LiveBuilds key={`${buildRequestKey(state.report.request)}:${state.report.meta.published_at}`} report={state.report} records={records} locale={locale} customGame={session.context?.customGame === true} onOpen={setDetail}/>
      : <div className="surface live-build-status" role="status">
        <p>{state?.status === 'error' ? t.errors[state.error] : t.loading}</p>
        {state?.status === 'error' && <button className="button" onClick={retry}>{t.retry}</button>}
      </div>}
    {detail && <GameDetails record={detail} records={records} locale={locale} version={catalog.version} onClose={() => setDetail(null)} onOpen={setDetail}/>}
  </>;
}

export function InGameScreen({locale}: {locale: Locale}) {
  const {session, native, error, retry} = useLiveSession();
  const t = liveCopy[locale];
  const context = liveBuildContext(session);
  // La génération et le contexte remettent à zéro catalogues, requêtes et dialogues.
  const workspaceKey = session && context ? `${session.generation}:${JSON.stringify(context)}:${locale}` : null;
  return <div className="screen live-game-screen">
    <header className="screen-heading"><h1>{t.title}</h1></header>
    {!native ? <p className="surface live-build-status" role="status">{t.desktopRequired}</p>
      : error ? <div className="surface live-build-status" role="status"><p>{t.connectionError}</p><button className="button" onClick={retry}>{buildCopy[locale].retry}</button></div>
        : <>
          <div className="surface live-game-summary"><LiveSummary session={session} locale={locale}/></div>
          {workspaceKey && context && session ? <LiveBuildWorkspace key={workspaceKey} session={session} locale={locale} championId={context.champion_id}/>
            : session?.status === 'ready' && <p className="surface live-build-status" role="status">{t.contextMissing}</p>}
        </>}
  </div>;
}
