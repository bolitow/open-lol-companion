import {rankLabel} from '../buildRanks';
import {usePreparation} from '../PreparationContext';
import {memo, useEffect, useState} from 'react';
import type {BuildReport, BuildStats, CatalogRecord, LiveSession} from '@olc/shared';
import type {Locale} from '../state';
import {buildMetrics, variantsFor} from '../buildModel';
import {buildCopy} from '../buildCopy';
import {championDetails} from '../draft';
import {useBuilds} from '../useBuilds';
import {connectLiveCatalog, type LiveCatalogState} from './liveCatalog';
import {liveBuildContext, liveBuildRequest} from './liveModel';
import {liveCopy} from './liveCopy';
import './liveBuildSummary.css';

const copy = {
  fr: {title: 'Variantes les plus jouées', keystone: 'Fondamentale', observed: 'Victoire observée', custom: 'Partie personnalisée · source Solo/Duo', source: 'Source : API Open LoL Companion'},
  en: {title: 'Most-played variants', keystone: 'Keystone', observed: 'Observed win rate', custom: 'Custom game · Solo/Duo source', source: 'Source: Open LoL Companion API'},
} as const;

function findRecord(records: CatalogRecord[], id: number | undefined, kind: string, locale: Locale): CatalogRecord | null {
  const matches = records.filter(record => record.id === String(id) && record.kind === kind && record.namespace === 'standard' && record.locale === (locale === 'fr' ? 'fr_FR' : 'en_US'));
  return matches.length === 1 ? matches[0]! : null;
}

function Keystone({variant, records, locale}: {variant: BuildStats; records: CatalogRecord[]; locale: Locale}) {
  const record = findRecord(records, variant.selection[1], 'rune', locale);
  const field = (name: string) => {
    const value = record?.fields[name];
    return value && ['verified', 'derived'].includes(value.status) ? value.value : null;
  };
  const valid = variant.selection.length === 11 && field('rune_kind') === 'rune' && field('slot') === 0 && field('style_id') === String(variant.selection[0]);
  return <CompactRecord record={valid ? record : null} locale={locale}/>;
}

function CompactRecord({record, locale}: {record: CatalogRecord | null; locale: Locale}) {
  return record ? record.icon ? <img src={record.icon} alt={record.name} title={record.name}/> : <span>{record.name}</span>
    : <span className="live-build-unknown" title={buildCopy[locale].unknown} aria-label={buildCopy[locale].unknown}>—</span>;
}

function CompactMetrics({variant, minimum, locale}: {variant: BuildStats; minimum: number; locale: Locale}) {
  // Seul le taux déjà publié est repris ; le seuil de publication reste signalé séparément.
  const metrics = buildMetrics(variant, 1);
  const t = buildCopy[locale];
  const percent = metrics.winRate === null ? '—' : `${new Intl.NumberFormat(locale, {maximumFractionDigits: 1}).format(metrics.winRate)} %`;
  return <div className="live-build-compact-metrics">
    <span>{new Intl.NumberFormat(locale).format(metrics.games)} {t.games}</span>
    <span>{copy[locale].observed} {percent}</span>
    {metrics.games < minimum && <small>{t.sample}</small>}
  </div>;
}

/** Deux catégories indépendantes, sans adaptation aux événements ni commande de jeu. */
export const LiveBuildSummaryContent = memo(function LiveBuildSummaryContent({report, records, locale, customGame}: {report: BuildReport; records: CatalogRecord[]; locale: Locale; customGame: boolean}) {
  const t = buildCopy[locale], labels = copy[locale];
  const items = variantsFor(report, 'final_items')[0];
  const runes = variantsFor(report, 'runes')[0];
  const {request} = report;
  const champion = championDetails(request.champion_id, locale);
  const queue = t.queues[request.queue as keyof typeof t.queues] ?? new Intl.NumberFormat(locale, {useGrouping: false}).format(request.queue);
  return <section className="live-build-compact" aria-label={labels.title}>
    <header><strong>{champion?.name ?? liveCopy[locale].unknownChampion} · {t.roles[request.role]}</strong><span>{request.platform} · {queue} · {t.patch} {request.patch}</span></header>
    {customGame && <small>{labels.custom}</small>}
    <div className="live-build-compact-items"><h3>{t.final_items}</h3>
      {items ? <>
        {items.selection.length ? <ul aria-label={t.final_items}>{items.selection.slice(0, 6).map((id, index) => <li key={`${id}:${index}`}><CompactRecord record={findRecord(records, id, 'item', locale)} locale={locale}/></li>)}</ul> : <p>{t.emptySelection}</p>}
        <CompactMetrics variant={items} minimum={report.meta.min_games} locale={locale}/>
      </> : <p>{t.categoryMissing}</p>}
    </div>
    {runes && <div className="live-build-compact-rune"><h3>{labels.keystone}</h3><div className="live-build-compact-rune-row"><Keystone variant={runes} records={records} locale={locale}/><CompactMetrics variant={runes} minimum={report.meta.min_games} locale={locale}/></div></div>}
    <footer><span>{t.independent}</span><span>{labels.source}</span></footer>
  </section>;
});

function LiveBuildSummaryLoader({session, locale, championId}: {session: LiveSession; locale: Locale; championId: number}) {
  const [catalogState, setCatalogState] = useState<LiveCatalogState>({catalog: null, abilities: [], error: false});
  useEffect(() => connectLiveCatalog(locale, championId, setCatalogState), [locale, championId]);
  const catalog = catalogState.catalog;
  const {value:preparation,defaultRank='EMERALD_PLUS'}=usePreparation();
  const request = catalog ? liveBuildRequest(session, catalog.version,preparation.rankOverride??defaultRank) : null;
  const {state} = useBuilds(request);
  if (!catalog) return <p className="live-build-compact-status" role="status">{catalogState.error ? liveCopy[locale].catalogError : liveCopy[locale].catalogLoading}</p>;
  if (state?.status !== 'ready') return <p className="live-build-compact-status" role="status">{state?.status === 'error' ? buildCopy[locale].errors[state.error] : buildCopy[locale].loading}</p>;
  return <LiveBuildSummaryContent report={state.report} records={catalog.records} locale={locale} customGame={session.context?.customGame === true}/>;
}

export function LiveBuildSummary({session, locale}: {session: LiveSession | null; locale: Locale}) {
  const context = liveBuildContext(session);
  if (!session || !context) return <p className="live-build-compact-status" role="status">{liveCopy[locale].contextMissing}</p>;
  return <LiveBuildSummaryLoader key={`${session.generation}:${JSON.stringify(context)}:${locale}`} session={session} locale={locale} championId={context.champion_id}/>;
}
