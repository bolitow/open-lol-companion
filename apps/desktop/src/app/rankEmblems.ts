// Emblèmes distribués par Riot : https://developer.riotgames.com/docs/lol#ranked-info_icons-and-emblems
const tiers = new Set(['IRON','BRONZE','SILVER','GOLD','PLATINUM','EMERALD','DIAMOND','MASTER','GRANDMASTER','CHALLENGER']);
export const rankEmblemUrl = (tier: string|null|undefined): string|null => tier && tiers.has(tier) ? `/game-data/ranks/${tier.toLowerCase()}.png` : null;
