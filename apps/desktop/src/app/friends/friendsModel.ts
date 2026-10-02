import type {PlayerRequest} from '@olc/shared';
import type {Friend} from '@olc/shared';
import {parsePlayerQuery} from '../playerStore';

export function friendPlayerRequest(friend: Friend): PlayerRequest | null {
    if (!friend.game_name || !friend.tag_line || !friend.platform) return null;
    return parsePlayerQuery(`${friend.game_name}#${friend.tag_line}`, friend.platform);
}
export function availableFriends(friends: readonly Friend[]): number {
    return friends.filter(friend => ['online', 'away', 'busy'].includes(friend.presence)).length;
}

/** Une identité complète garde son élément DOM après insertion ou changement de présence. */
export function friendRowKey(friend:Friend,index:number):string {
 const identity=friendPlayerRequest(friend);
 return identity?JSON.stringify([identity.platform,identity.game_name,identity.tag_line]):JSON.stringify([friend.name,index]);
}
