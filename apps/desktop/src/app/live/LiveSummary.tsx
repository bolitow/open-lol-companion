import type {LiveSession} from '@olc/shared';
import type {Locale} from '../state';
import {formatLiveTime, liveChampion} from './liveModel';
import {liveCopy} from './liveCopy';
import './liveSummary.css';

/** Projection passive commune à l'écran principal et à la fenêtre d'overlay. */
export function LiveSummary({session, locale}: {session: LiveSession | null; locale: Locale}) {
  const t = liveCopy[locale];
  if (session?.status !== 'ready' || !session.game) {
    const status = session?.status === 'ready' ? 'waiting' : session?.status ?? 'waiting';
    return <p className="live-summary-status" role="status">{t.statuses[status]}</p>;
  }
  const {player, gameTime} = session.game;
  const champion = liveChampion(session, locale);
  const number = new Intl.NumberFormat(locale);
  return <section className="live-summary" aria-label={t.player}>
    <div className="live-summary-champion">
      {champion && <img src={champion.image} alt=""/>}
      <div><strong>{champion?.name ?? t.unknownChampion}</strong><span>{t.level} {number.format(player.level)}</span></div>
    </div>
    <dl className="live-summary-values">
      <div><dt>{t.kda}</dt><dd>{`${number.format(player.kills)} / ${number.format(player.deaths)} / ${number.format(player.assists)}`}</dd></div>
      <div><dt>{t.cs}</dt><dd>{number.format(player.creepScore)}</dd></div>
      <div><dt>{t.time}</dt><dd>{formatLiveTime(gameTime, locale)}</dd></div>
    </dl>
    <small className="live-summary-cs-note">{t.csHint}</small>
  </section>;
}
