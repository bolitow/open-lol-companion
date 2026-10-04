import { championIconUrl, itemIconUrl, type BuildStats } from "@olc/shared";
import type { Metadata } from "next";
import { notFound } from "next/navigation";
import type { ReactNode } from "react";
import { ErrorNotice } from "../../../../components/ErrorNotice";
import { SnapshotNotes } from "../../../../components/SnapshotNotes";
import { StatsFilterForm } from "../../../../components/StatsFilterForm";
import { decodeRunes, skillLetters, topVariants } from "../../../../lib/builds";
import { championBySlug } from "../../../../lib/champions";
import { filtersHref, parseStatsFilters, patchOptions, type SearchParams } from "../../../../lib/filters";
import { formatCount, formatRate } from "../../../../lib/format";
import { dictionary, interpolate, isLocale, type Dictionary, type Locale } from "../../../../lib/i18n";
import { runeIconUrl, spellIconUrl } from "../../../../lib/images";
import { getApi, loadGameData, loadStatic } from "../../../../lib/server";

type Props = { params: Promise<{ locale: string; slug: string }>; searchParams: Promise<SearchParams> };

export async function generateMetadata({ params }: Props): Promise<Metadata> {
  const { locale, slug } = await params;
  if (!isLocale(locale)) return {};
  const statics = await loadStatic(locale);
  const champion = statics ? championBySlug(statics.champions, slug) : undefined;
  if (!champion) return {};
  const t = dictionary(locale).champion;
  return {
    title: `${champion.name} : ${t.titleSuffix}`,
    alternates: { languages: { fr: `/fr/champions/${slug}`, en: `/en/champions/${slug}` } },
  };
}

type GameData = Awaited<ReturnType<typeof loadGameData>>;

function VariantStats({ build, locale, t }: { build: BuildStats; locale: Locale; t: Dictionary["champion"] }) {
  return (
    <span className="stats">
      {interpolate(t.games, { count: formatCount(build.games, locale) })}
      {build.pick_rate !== null ? ` · ${interpolate(t.variantShare, { share: formatRate(build.pick_rate, locale) })}` : null}
      {build.performance_available && build.win_rate !== null
        ? ` · ${interpolate(t.variantWin, { rate: formatRate(build.win_rate, locale) })}`
        : null}
    </span>
  );
}

function Section({ title, builds, render, locale, t }: {
  title: string;
  builds: BuildStats[];
  render: (build: BuildStats) => ReactNode;
  locale: Locale;
  t: Dictionary["champion"];
}) {
  return (
    <section className="card">
      <h2>{title}</h2>
      {builds.length === 0 ? (
        <p className="muted">{t.noVariants}</p>
      ) : (
        builds.map((build) => (
          <div className="variant" key={build.selection.join("-")}>
            {render(build)}
            <VariantStats build={build} locale={locale} t={t} />
          </div>
        ))
      )}
    </section>
  );
}

function ItemIcons({ ids, version, data }: { ids: number[]; version: string; data: GameData }) {
  return ids.map((id, index) => {
    const name = data.items.get(id)?.name ?? `#${id}`;
    return <img key={`${id}-${index}`} className="icon" src={itemIconUrl(version, id)} alt={name} title={name} width={32} height={32} loading="lazy" />;
  });
}

export default async function ChampionPage({ params, searchParams }: Props) {
  const { locale, slug } = await params;
  if (!isLocale(locale)) notFound();
  const t = dictionary(locale);
  const statics = await loadStatic(locale);
  if (!statics) {
    return <ErrorNotice locale={locale} code="unavailable" />;
  }
  const champion = championBySlug(statics.champions, slug);
  if (!champion) notFound();
  const filters = parseStatsFilters(await searchParams, { patch: statics.patch });
  const patch = filters.patch ?? statics.patch;
  const path = `/${locale}/champions/${slug}`;
  const [result, data] = await Promise.all([
    getApi().builds({ ...filters, patch }, champion.key),
    loadGameData(locale, statics.version),
  ]);
  const summary = result.ok ? result.data.summary : null;
  const builds = result.ok ? result.data.builds : [];
  const published = result.ok ? result.data.meta.filters.patches : [];

  return (
    <>
      <p>
        <a href={filtersHref(`/${locale}/tierlist`, filters)}>{t.champion.back}</a>
      </p>
      <h1 className="champion-cell">
        <img className="icon" src={championIconUrl(statics.version, champion.id)} alt="" width={48} height={48} />
        {champion.name}
      </h1>
      {champion.title ? <p className="muted">{champion.title}</p> : null}
      <StatsFilterForm locale={locale} path={path} filters={filters} patches={patchOptions([filters.patch, statics.patch], [published])} />
      {!result.ok ? (
        <ErrorNotice locale={locale} code={result.code} />
      ) : summary === null ? (
        <p className="notice">{t.champion.noData}</p>
      ) : (
        <>
          <section aria-label={t.champion.summary}>
            <dl className="summary">
              <div>
                <dt>{t.tierlist.tier}</dt>
                <dd>{summary.tier ?? "—"}</dd>
              </div>
              <div>
                <dt>{t.tierlist.winRate}</dt>
                <dd>{formatRate(summary.win_rate, locale)}</dd>
              </div>
              <div>
                <dt>{t.tierlist.pickRate}</dt>
                <dd>{formatRate(summary.pick_rate, locale)}</dd>
              </div>
              <div>
                <dt>{t.tierlist.games}</dt>
                <dd>{formatCount(summary.games, locale)}</dd>
              </div>
            </dl>
          </section>
          <div className="grid">
            <Section
              title={t.champion.runes}
              builds={topVariants(builds, "runes", 3)}
              locale={locale}
              t={t.champion}
              render={(build) => {
                const runes = decodeRunes(build.selection);
                if (!runes) return null;
                return [runes.primaryStyle, ...runes.primary, runes.subStyle, ...runes.secondary].map((id) => {
                  const rune = data.runes.get(id);
                  return rune ? (
                    <img key={id} className="icon" src={runeIconUrl(rune.icon)} alt={rune.name} title={rune.name} width={28} height={28} loading="lazy" />
                  ) : (
                    <span key={id}>#{id}</span>
                  );
                });
              }}
            />
            <Section
              title={t.champion.spells}
              builds={topVariants(builds, "summoner_spells", 3)}
              locale={locale}
              t={t.champion}
              render={(build) =>
                build.selection.map((id) => {
                  const spell = data.spells.get(id);
                  return spell ? (
                    <img key={id} className="icon" src={spellIconUrl(statics.version, spell.id)} alt={spell.name} title={spell.name} width={32} height={32} loading="lazy" />
                  ) : (
                    <span key={id}>#{id}</span>
                  );
                })
              }
            />
            <Section
              title={t.champion.skillOrder}
              builds={topVariants(builds, "skill_order", 3)}
              locale={locale}
              t={t.champion}
              render={(build) =>
                skillLetters(build.selection).map((key, index) => (
                  <span key={index} className="skill">
                    {key}
                  </span>
                ))
              }
            />
            <Section
              title={t.champion.items}
              builds={topVariants(builds, "final_items", 5)}
              locale={locale}
              t={t.champion}
              render={(build) => <ItemIcons ids={build.selection} version={statics.version} data={data} />}
            />
            <Section
              title={t.champion.popularItems}
              builds={topVariants(builds, "item", 6)}
              locale={locale}
              t={t.champion}
              render={(build) => <ItemIcons ids={build.selection} version={statics.version} data={data} />}
            />
            <Section
              title={t.champion.trinket}
              builds={topVariants(builds, "trinket", 2)}
              locale={locale}
              t={t.champion}
              render={(build) => <ItemIcons ids={build.selection} version={statics.version} data={data} />}
            />
          </div>
        </>
      )}
      {result.ok ? <SnapshotNotes locale={locale} meta={result.data.meta} /> : null}
    </>
  );
}
