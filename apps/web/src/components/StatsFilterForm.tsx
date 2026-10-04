import { filtersHref, PLATFORMS, QUEUES, RANKS, ROLES, type StatsFilters } from "../lib/filters";
import { dictionary, type Locale } from "../lib/i18n";

interface Props {
  locale: Locale;
  path: string;
  filters: StatsFilters;
  /** Patches proposés : publication courante et patch demandé. */
  patches: string[];
}

/** Rôles en onglets, autres filtres en formulaire GET : utilisable sans JavaScript. */
export function StatsFilterForm({ locale, path, filters, patches }: Props) {
  const t = dictionary(locale).filters;
  return (
    <>
      <nav aria-label={t.role}>
        <ul className="tabs">
          {ROLES.map((role) => (
            <li key={role}>
              <a href={filtersHref(path, filters, { role })} aria-current={role === filters.role ? "page" : undefined}>
                {t.roles[role]}
              </a>
            </li>
          ))}
        </ul>
      </nav>
      <form method="get" action={path}>
        <fieldset className="filters">
          <legend className="sr-only">{t.legend}</legend>
          <input type="hidden" name="role" value={filters.role} />
          <label>
            {t.rank}
            <select name="rank" defaultValue={filters.rank}>
              {RANKS.map((rank) => (
                <option key={rank} value={rank}>
                  {t.ranks[rank]}
                </option>
              ))}
            </select>
          </label>
          <label>
            {t.platform}
            <select name="platform" defaultValue={filters.platform}>
              {PLATFORMS.map((platform) => (
                <option key={platform} value={platform}>
                  {platform}
                </option>
              ))}
            </select>
          </label>
          <label>
            {t.queue}
            <select name="queue" defaultValue={String(filters.queue)}>
              {QUEUES.map((queue) => (
                <option key={queue} value={queue}>
                  {t.queues[queue]}
                </option>
              ))}
            </select>
          </label>
          {patches.length > 0 ? (
            <label>
              {t.patch}
              <select name="patch" defaultValue={filters.patch ?? undefined}>
                {patches.map((patch) => (
                  <option key={patch} value={patch}>
                    {patch}
                  </option>
                ))}
              </select>
            </label>
          ) : null}
          <button type="submit">{t.apply}</button>
        </fieldset>
      </form>
    </>
  );
}
