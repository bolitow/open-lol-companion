import {useId,useState} from 'react';
import type {PlayerRequest} from '@olc/shared';
import type {Friend, FriendsState} from '@olc/shared';
import type {Locale} from '../state';
import {Icon} from '../../ui/Icon';
import {availableFriends, friendPlayerRequest, friendRowKey} from './friendsModel';
import {friendsCopy} from './friendsCopy';
import './friends.css';
import {profileIconUrl} from '../AccountControl';

export interface FriendsPanelProps {locale: Locale; state: FriendsState; onPlayer: (request: PlayerRequest) => void}
export function FriendRow({locale, friend, onPlayer}: {locale: Locale; friend: Friend; onPlayer: FriendsPanelProps['onPlayer']}) {
    const t = friendsCopy[locale], request = friendPlayerRequest(friend);
    const name = friend.name.trim() || friend.game_name?.trim() || t.unknownFriend;
    const detail = `${name}${request ? ` · ${request.game_name}#${request.tag_line} · ${request.platform}` : ''} · ${t.presence[friend.presence]}${request ? '' : ` · ${t.profileUnavailable}`}`;
    const url = profileIconUrl(friend.icon_id);
    const content = <>
        <span className="friend-avatar" aria-hidden="true"><FriendAvatar key={url} url={url} name={name} locale={locale}/><span className={`friend-presence-dot presence-${friend.presence}`}/></span>
        <span className="friend-details"><strong>{name}</strong></span>
        {request && <Icon name="chevron" size={14}/>}
    </>;
    return request
        ? <button type="button" className="friend-row" title={detail} onClick={() => onPlayer(request)} aria-label={`${t.openProfile} ${detail}`}>{content}</button>
        : <div className="friend-row" role="group" title={detail} aria-label={detail}>{content}</div>;
}
function FriendAvatar({url,name,locale}:{url:string|null;name:string;locale:Locale}) {
    const [failed,setFailed]=useState(false);
    return url&&!failed ? <img src={url} alt="" loading="lazy" referrerPolicy="no-referrer" onError={()=>setFailed(true)}/> : <span>{Array.from(name).slice(0,2).join('').toLocaleUpperCase(locale)}</span>;

}
export function FriendsPanel({locale, state, onPlayer}: FriendsPanelProps) {
    const t = friendsCopy[locale], titleId = useId();
    // Un état transitoire ne doit jamais réafficher la liste du compte précédent.
    const items = state.status === 'ready' ? state.items : [];
    return <section className="surface friends-panel" aria-labelledby={titleId} aria-busy={state.status === 'loading'}>
        <header><div><Icon name="users" size={20}/><h2 id={titleId}>{t.title}</h2></div>
            {state.status === 'ready' && <small>{t.count(availableFriends(items), items.length)}</small>}
        </header>
        {state.status !== 'ready' ? <p className="friends-message" role="status">{t.states[state.status]}</p>
            : items.length === 0 ? <p className="friends-message" role="status">{t.empty}</p>
            : <div className="friends-scroll" role="region" aria-labelledby={titleId} tabIndex={0}>
                <ul>{items.map((friend, index) => <li key={friendRowKey(friend,index)}>
                    <FriendRow friend={friend} locale={locale} onPlayer={onPlayer}/>
                </li>)}</ul>
            </div>}
    </section>;
}
