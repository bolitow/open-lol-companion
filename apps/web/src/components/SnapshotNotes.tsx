import type { SnapshotMeta } from "@olc/shared";
import { formatCount, formatDate } from "../lib/format";
import { dictionary, interpolate, type Locale } from "../lib/i18n";

/** Seuils et méthodes de la publication ; les textes techniques de l'API restent en anglais, balisés comme tels. */
export function SnapshotNotes({ locale, meta, bans = false }: { locale: Locale; meta: SnapshotMeta; bans?: boolean }) {
  const t = dictionary(locale).stats;
  return (
    <div className="notes">
      <p>
        {interpolate(t.published, { date: formatDate(meta.published_at, locale) })}{" "}
        {interpolate(t.threshold, { count: formatCount(meta.min_games, locale) })}
        {bans ? ` ${t.bans}` : null}
      </p>
      <p>
        {t.tierMethod} <code lang="en">{meta.tier_method}</code>
      </p>
      <p>
        {t.pickRate} <code lang="en">{meta.pick_rate_definition}</code>
      </p>
    </div>
  );
}
