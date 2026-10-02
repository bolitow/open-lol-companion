import {InGameScreen} from './live/InGameScreen';
import type {ReactNode} from 'react';
import type { LcuSession } from '@olc/shared';
import { Flame, Icon } from '../ui/Icon';
import type { Copy } from './copy';
import type { Screen,Locale } from './state';
import {DraftBoard} from './DraftBoard';
export function EmptyPanel({ title, description, icon = 'chart' }: {
    title: string;
    description: string;
    icon?: 'chart' | 'users' | 'shield' | 'sword';
}) {
    return <section className="surface empty-panel"><header><Icon name={icon}/><h2>{title}</h2></header><div className="empty-lines" aria-hidden="true"><i /><i /><i /></div><p>{description}</p></section>;
}
export function Dashboard({ t, onDraft, companion, profile, history, friends }: {
    t: Copy;
    onDraft: () => void;
    companion: ReactNode;
    profile: ReactNode;
    history: ReactNode;
    friends: ReactNode;
}) {
    return <div className="dashboard-layout">
      <div className="dashboard-main">
        <div className="dashboard-summary">
          {profile}
          <section className="welcome surface"><div className="welcome-copy"><span className="eyebrow">{t.home.eyebrow}</span><h1>{t.home.title}</h1><p>{t.home.description}</p><button className="button primary" onClick={onDraft}>{t.home.openDraft}<Icon name="arrow" size={18}/></button></div><div className="welcome-companion">{companion}</div></section>
        </div>
        {history}
      </div>
      <aside className="dashboard-aside">{friends}<EmptyPanel title={t.home.progress} description={t.home.progressHint}/></aside>
    </div>;
}
export function GameScreen({ screen, session, t, locale }: {
    screen: Exclude<Screen, 'settings' | 'dashboard' | 'champions' | 'players'>;
    locale:Locale;
    session: LcuSession;
    t: Copy;
}) {
    if (screen === 'in-game') return <InGameScreen locale={locale}/>;
    if (screen !== 'champ-select')
        return <><div className="screen-heading"><span className="eyebrow">{session.phase ? t.phases[session.phase] : t.game.phaseUnknown}</span><h1>{t.navigation[screen]}</h1><p>{t.game.postHint}</p></div><EmptyPanel title={t.game.unavailable} description={t.connection.data} icon="sword"/></>;
    return <DraftBoard connected={session.connected} draft={session.draft} runePage={session.runePage} locale={locale} t={t}/>;
}
