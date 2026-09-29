import { useEffect, useState } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import type { LcuStatus } from "@olc/shared";

const POLL_MS = 3000;

/** Écran de démarrage : indique si le client League of Legends est détecté (issue « Connecteur LCU »). */
export function App() {
  const [status, setStatus] = useState<LcuStatus | null>(null);

  useEffect(() => {
    if (!isTauri()) {
      setStatus({ connected: false, port: null, message: "Lancez l'app avec `pnpm dev` pour détecter le client." });
      return;
    }
    const check = () => invoke<LcuStatus>("lcu_status").then(setStatus).catch(console.error);
    check();
    const id = setInterval(check, POLL_MS);
    return () => clearInterval(id);
  }, []);

  return (
    <main className="app">
      <h1>Open LoL Companion</h1>
      <p className="subtitle">Version de développement</p>
      <section className={`status ${status?.connected ? "ok" : "off"}`}>
        <span className="dot" aria-hidden />
        <div>
          <strong>{status?.connected ? "Client League of Legends détecté" : "Client non détecté"}</strong>
          <p>{status?.message ?? "Recherche…"}</p>
        </div>
      </section>
    </main>
  );
}
