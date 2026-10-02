export type Theme = "dark" | "light";
export type Locale = "fr" | "en";
export type AccountId = "nebuleuse" | "orbite";
export type QueueFilter = "all" | "ranked" | "aram";
export type HistoryState = "ready" | "loading" | "empty" | "error";
export interface Preferences { theme: Theme; locale: Locale; motion: boolean }
export const initialSession = { account: "nebuleuse" as AccountId, connected: true, selectedMatch: null as string | null };
export type Session = typeof initialSession;
export type SessionAction = { type: "connection"; connected: boolean } | { type: "account"; account: AccountId } | { type: "match"; id: string | null };
export function sessionReducer(state: Session, action: SessionAction): Session {
  switch (action.type) {
    case "connection": return { ...state, connected: action.connected };
    case "account": return { account: action.account, connected: true, selectedMatch: null };
    case "match": return { ...state, selectedMatch: action.id };
  }
}
export function parsePreferences(raw: string | null): Preferences {
  const defaults: Preferences = { theme: "dark", locale: "fr", motion: true };
  try {
    const value: unknown = JSON.parse(raw ?? "null");
    if (!value || typeof value !== "object") return defaults;
    const data = value as Record<string, unknown>;
    return {
      theme: data.theme === "light" ? "light" : "dark",
      locale: data.locale === "en" ? "en" : "fr",
      motion: typeof data.motion === "boolean" ? data.motion : true,
    };
  } catch { return defaults; }
}
export function filterMatches<T extends { account: string; queue: string }>(rows: T[], account: AccountId, queue: QueueFilter): T[] {
  return rows.filter(row => row.account === account && (queue === "all" || row.queue === queue));
}
export function searchCatalog<T extends { label: string; keywords: string }>(entries: T[], query: string): T[] {
  const normalize = (value: string) => value.normalize("NFD").replace(/[\u0300-\u036f]/g, "").toLowerCase().trim();
  const term = normalize(query);
  return entries.filter(entry => normalize(`${entry.label} ${entry.keywords}`).includes(term));
}
export function pageFromHash(hash:string):"home"|"draft"|"layouts" {
  return hash==="#draft"?"draft":hash==="#layouts"?"layouts":"home";
}
