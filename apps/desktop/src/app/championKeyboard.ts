import type {ChampionsState} from './state';
export const profileTabs=['abilities','builds','catalog'] as const;
export function nextProfileTab(current:ChampionsState['tab'],key:string):ChampionsState['tab']|null{
 if(key==='Home')return profileTabs[0];if(key==='End')return profileTabs[2];
 if(key!=='ArrowLeft'&&key!=='ArrowRight')return null;
 return profileTabs[(profileTabs.indexOf(current)+(key==='ArrowRight'?1:2))%3]!;
}
