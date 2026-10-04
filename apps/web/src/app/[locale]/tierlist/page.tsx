import { championIconUrl } from "@olc/shared";
import type { Metadata } from "next";
import { notFound } from "next/navigation";
import { ErrorNotice } from "../../../components/ErrorNotice";
import { SnapshotNotes } from "../../../components/SnapshotNotes";
import { StatsFilterForm } from "../../../components/StatsFilterForm";
import { championSlug, tierlistRows } from "../../../lib/champions";
import { filtersHref, parseStatsFilters, patchOptions, type SearchParams } from "../../../lib/filters";
import { formatCount, formatRate } from "../../../lib/format";
import { dictionary, interpolate, isLocale } from "../../../lib/i18n";
import { getApi, loadStatic } from "../../../lib/server";

type Props = { params: Promise<{ locale: string }>; searchParams: Promise<SearchParams> };

export async function generateMetadata({ params }: Props): Promise<Metadata> {
  const { locale } = await params;
  if (!isLocale(locale)) return {};
  const t = dictionary(locale).tierlist;
  return {
    title: t.title,
    description: t.intro,
    alternates: { languages: { fr: "/fr/tierlist", en: "/en/tierlist" } },
  };
}

export default async function TierlistPage({ params, searchParams }: Props) {
  const { locale } = await params;
  if (!isLocale(locale)) notFound();
  const t = dictionary(locale);
  const statics = await loadStatic(locale);
  const filters = parseStatsFilters(await searchParams, { patch: statics?.patch ?? null });
  const path = `/${locale}/tierlist`;

  if (filters.patch === null) {
    return (
      <>
        <h1>{t.tierlist.title}</h1>
        <p className="notice">{t.stats.noPatch}</p>
      </>
    );
  }

  const result = await getApi().tierlist({ ...filters, patch: filters.patch });
  const published = result.ok ? result.data.meta.filters.patches : [];
  const patches = patchOptions([filters.patch, statics?.patch ?? null], [published]);
  const rows = result.ok ? tierlistRows(result.data, statics?.champions ?? new Map()) : [];

  return (
    <>
      <h1>{t.tierlist.title}</h1>
      <p className="muted">{t.tierlist.intro}</p>
      <StatsFilterForm locale={locale} path={path} filters={filters} patches={patches} />
      {!result.ok ? (
        <ErrorNotice locale={locale} code={result.code} />
      ) : rows.length === 0 ? (
        <div className="notice">
          <p>{t.tierlist.empty}</p>
          {published.length > 0 && !published.includes(filters.patch) ? (
            <p>
              {t.stats.otherPatches}{" "}
              {published.map((patch, index) => (
                <span key={patch}>
                  {index > 0 ? ", " : null}
                  <a href={filtersHref(path, filters, { patch })}>{patch}</a>
                </span>
              ))}
            </p>
          ) : null}
        </div>
      ) : (
        <div className="table-wrap">
          <table>
            <caption>
              {interpolate(t.tierlist.caption, {
                role: t.filters.roles[filters.role],
                rank: t.filters.ranks[filters.rank],
                platform: filters.platform,
                patch: filters.patch,
              })}
            </caption>
            <thead>
              <tr>
                <th scope="col">{t.tierlist.position}</th>
                <th scope="col">{t.tierlist.champion}</th>
                <th scope="col">{t.tierlist.tier}</th>
                <th scope="col">{t.tierlist.winRate}</th>
                <th scope="col">{t.tierlist.pickRate}</th>
                <th scope="col">{t.tierlist.banRate}</th>
                <th scope="col">{t.tierlist.games}</th>
              </tr>
            </thead>
            <tbody>
              {rows.map((row) => {
                const label = row.name ?? interpolate(t.tierlist.unknownChampion, { id: String(row.championId) });
                return (
                  <tr key={row.championId}>
                    <td>{row.position ?? "—"}</td>
                    <th scope="row">
                      {row.iconId && statics ? (
                        <a className="champion-cell" href={filtersHref(`/${locale}/champions/${championSlug({ id: row.iconId })}`, filters)}>
                          <img className="icon" src={championIconUrl(statics.version, row.iconId)} alt="" width={32} height={32} loading="lazy" />
                          {label}
                        </a>
                      ) : (
                        label
                      )}
                    </th>
                    <td>{row.tier ?? "—"}</td>
                    <td>{formatRate(row.winRate, locale)}</td>
                    <td>{formatRate(row.pickRate, locale)}</td>
                    <td>{formatRate(row.banRate, locale)}</td>
                    <td>{formatCount(row.games, locale)}</td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      )}
      {result.ok ? <SnapshotNotes locale={locale} meta={result.data.meta} bans /> : null}
    </>
  );
}
