import { championIconUrl, itemIconUrl } from "@olc/shared";
import type { Metadata } from "next";
import { notFound } from "next/navigation";
import { ErrorNotice } from "../../../../../../components/ErrorNotice";
import { isPlatform, ROLES, type SearchParams } from "../../../../../../lib/filters";
import { formatDate, formatDuration } from "../../../../../../lib/format";
import { dictionary, interpolate, isLocale, type Dictionary } from "../../../../../../lib/i18n";
import { profileIconUrl } from "../../../../../../lib/images";
import { parseHistoryStart, rankLabel, riotIdFromSegments } from "../../../../../../lib/profile";
import { profilePath } from "../../../../../../lib/riotId";
import { getApi, loadStatic } from "../../../../../../lib/server";

type Props = {
  params: Promise<{ locale: string; platform: string; gameName: string; tagLine: string }>;
  searchParams: Promise<SearchParams>;
};

export async function generateMetadata({ params }: Props): Promise<Metadata> {
  const { locale, gameName, tagLine } = await params;
  if (!isLocale(locale)) return {};
  const id = riotIdFromSegments(gameName, tagLine);
  return {
    title: id ? `${id.gameName}#${id.tagLine} : ${dictionary(locale).profile.titleSuffix}` : undefined,
    // Profils consultés à la demande : pas d'indexation par les moteurs de recherche.
    robots: { index: false, follow: false },
  };
}

function queueName(queueId: number, t: Dictionary): string {
  const queues: Record<string, string> = t.profile.queues;
  return queues[String(queueId)] ?? interpolate(t.profile.otherQueue, { id: String(queueId) });
}

function roleName(role: string | null, t: Dictionary): string | null {
  const known = ROLES.find((value) => value === role);
  return known ? t.filters.roles[known] : null;
}

export default async function ProfilePage({ params, searchParams }: Props) {
  const { locale, platform, gameName, tagLine } = await params;
  if (!isLocale(locale)) notFound();
  const t = dictionary(locale);
  const id = riotIdFromSegments(gameName, tagLine);
  if (!id || !isPlatform(platform)) {
    return <p className="notice">{t.profile.invalid}</p>;
  }
  const start = parseHistoryStart((await searchParams).start);
  const api = getApi();
  const [profile, history, statics] = await Promise.all([
    api.profile(platform, id),
    api.matches(platform, id, start),
    loadStatic(locale),
  ]);
  if (!profile.ok) return <ErrorNotice locale={locale} code={profile.code} />;
  const path = profilePath(locale, platform, id);
  const p = profile.data;

  return (
    <>
      <h1 className="champion-cell">
        {statics && p.profile_icon_id !== null ? (
          <img className="icon" src={profileIconUrl(statics.version, p.profile_icon_id)} alt="" width={56} height={56} />
        ) : null}
        <span>
          {p.game_name}
          <span className="muted">#{p.tag_line}</span>
        </span>
      </h1>
      <p className="muted">
        {p.platform}
        {p.summoner_level !== null ? ` · ${interpolate(t.profile.level, { level: String(p.summoner_level) })}` : null}
        {" · "}
        {interpolate(t.profile.fetchedAt, { date: formatDate(p.fetched_at * 1000, locale) })}
      </p>
      <section className="card">
        <h2>{t.profile.ranks}</h2>
        {p.ranks.map((rank) => (
          <p key={rank.queue_id}>
            <strong>{queueName(rank.queue_id, t)}</strong> : {rankLabel(rank, t)}
          </p>
        ))}
        <p className="notes">{t.profile.notProvided}</p>
      </section>
      <section>
        <h2>{t.profile.history}</h2>
        {!history.ok ? (
          <ErrorNotice locale={locale} code={history.code} />
        ) : (
          <>
            {history.data.matches.length === 0 ? <p className="muted">{t.profile.empty}</p> : null}
            {history.data.matches.map((match) => {
              const champion = statics?.champions.get(match.champion_id);
              const role = roleName(match.role, t);
              return (
                <article key={match.match_id} className={match.win ? "match win" : "match"}>
                  <span className="result">{match.win ? t.profile.win : t.profile.loss}</span>
                  {champion && statics ? (
                    <img className="icon" src={championIconUrl(statics.version, champion.id)} alt="" width={40} height={40} loading="lazy" />
                  ) : null}
                  <span>
                    <strong>{champion?.name ?? interpolate(t.tierlist.unknownChampion, { id: String(match.champion_id) })}</strong>
                    <br />
                    <span className="muted">
                      {queueName(match.queue_id, t)}
                      {role ? ` · ${role}` : null}
                    </span>
                  </span>
                  {match.kills !== null && match.deaths !== null && match.assists !== null ? (
                    <span>
                      {interpolate(t.profile.kda, {
                        kills: String(match.kills),
                        deaths: String(match.deaths),
                        assists: String(match.assists),
                      })}
                    </span>
                  ) : null}
                  <span>
                    {statics
                      ? match.items
                          .filter((item) => item > 0)
                          .map((item, index) => (
                            <img key={`${item}-${index}`} className="icon" src={itemIconUrl(statics.version, item)} alt="" width={24} height={24} loading="lazy" />
                          ))
                      : null}
                  </span>
                  <span className="muted">
                    <span className="sr-only">{t.profile.duration} </span>
                    {formatDuration(match.duration_s)} · {formatDate(match.game_start_ms, locale)}
                  </span>
                </article>
              );
            })}
            {history.data.omitted_matches > 0 ? (
              <p className="notes">{interpolate(t.profile.omitted, { count: String(history.data.omitted_matches) })}</p>
            ) : null}
            <p>
              {start > 0 ? (
                <a className="button" href={path}>
                  {t.profile.first}
                </a>
              ) : null}{" "}
              {history.data.next_start !== null ? (
                <a className="button" href={`${path}?start=${history.data.next_start}`}>
                  {t.profile.next}
                </a>
              ) : null}
            </p>
          </>
        )}
      </section>
    </>
  );
}
