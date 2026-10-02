import type {BuildReport, CatalogRecord} from '@olc/shared';
import type {Locale} from '../state';
import {CommunityBuildPanels} from '../BuildPreparation';
import {buildCopy} from '../buildCopy';
import {liveCopy} from './liveCopy';

export function LiveBuilds({report, records, locale, customGame, onOpen}: {report: BuildReport; records: CatalogRecord[]; locale: Locale; customGame: boolean; onOpen: (record: CatalogRecord) => void}) {
  const t = buildCopy[locale];
  const {request} = report;
  const queue = t.queues[request.queue as keyof typeof t.queues] ?? new Intl.NumberFormat(locale, {useGrouping: false}).format(request.queue);
  const date = (value: string) => Number.isNaN(Date.parse(value)) ? '—' : new Intl.DateTimeFormat(locale, {dateStyle: 'short', timeStyle: 'short'}).format(new Date(value));
  return <div className="live-builds">
    <div className="live-build-scope">
      <strong>{liveCopy[locale].readOnly}</strong>
      <span>{request.platform} · {queue} · {t.roles[request.role]} · {t.all} · {t.patch} {request.patch}</span>
      {customGame && <p>{liveCopy[locale].customSource}</p>}
    </div>
    {report.builds.length
      ? <CommunityBuildPanels readOnly report={report} records={records} locale={locale} onOpen={onOpen}/>
      : <p className="surface live-build-status" role="status">{t.empty}</p>}
    <footer className="live-build-source">
      <span>{t.sources}</span><span>{t.collected} {date(report.meta.source_snapshot_at)} · {t.published} {date(report.meta.published_at)}</span>
      <p>{t.sourceHint}</p>
    </footer>
  </div>;
}
