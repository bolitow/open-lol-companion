"use client";

import { championIconUrl } from "@olc/shared";
import Link from "next/link";
import { useRouter } from "next/navigation";
import { useEffect, useMemo, useRef, useState, type FormEvent } from "react";
import { championSlug, type ChampionInfo } from "../lib/champions";
import { PLATFORMS, type Platform } from "../lib/filters";
import { interpolate, type Dictionary } from "../lib/i18n";
import { parseRiotId } from "../lib/riotId";
import { isSearchShortcut, searchChampions, searchTarget } from "../lib/search";

interface Props {
  locale: string;
  /** Version Data Dragon des icônes ; `null` si les statiques sont indisponibles. */
  version: string | null;
  champions: ChampionInfo[];
  copy: Dictionary["search"];
}

const MAX_RESULTS = 8;

/**
 * Recherche globale (Ctrl+K ou Cmd+K) : champions du catalogue statique et Riot ID saisi
 * par le joueur. Aucun appel réseau ici : le profil est lu par le serveur du site.
 */
export function SearchDialog({ locale, version, champions, copy }: Props) {
  const dialog = useRef<HTMLDialogElement>(null);
  const router = useRouter();
  const [query, setQuery] = useState("");
  const [platform, setPlatform] = useState<Platform>("EUW1");

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (isSearchShortcut(event)) {
        event.preventDefault();
        dialog.current?.showModal();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  const results = useMemo(() => searchChampions(query, champions, MAX_RESULTS), [query, champions]);
  const riotId = parseRiotId(query);

  const close = () => {
    dialog.current?.close();
    setQuery("");
  };

  const submit = (event: FormEvent) => {
    event.preventDefault();
    const target = searchTarget(query, platform, results, locale);
    if (target) {
      close();
      router.push(target);
    }
  };

  return (
    <>
      <button type="button" onClick={() => dialog.current?.showModal()} aria-haspopup="dialog">
        {copy.open}
        <kbd aria-hidden="true">{copy.shortcut}</kbd>
      </button>
      <dialog ref={dialog} className="search" aria-labelledby="search-title" onClose={() => setQuery("")}>
        <form onSubmit={submit} role="search">
          <h2 id="search-title" className="sr-only">
            {copy.title}
          </h2>
          <label htmlFor="global-search" className="sr-only">
            {copy.label}
          </label>
          <input
            id="global-search"
            type="search"
            autoComplete="off"
            autoFocus
            placeholder={copy.placeholder}
            value={query}
            onChange={(event) => setQuery(event.target.value)}
          />
          {riotId ? (
            <section aria-label={copy.player}>
              <p>
                <label>
                  {copy.platform}{" "}
                  <select value={platform} onChange={(event) => setPlatform(event.target.value as Platform)}>
                    {PLATFORMS.map((value) => (
                      <option key={value} value={value}>
                        {value}
                      </option>
                    ))}
                  </select>
                </label>
              </p>
              <button type="submit">
                {interpolate(copy.openProfile, { id: `${riotId.gameName}#${riotId.tagLine}`, platform })}
              </button>
            </section>
          ) : null}
          {query.trim() && !riotId ? (
            <section aria-label={copy.champions}>
              {results.length === 0 ? (
                <p className="muted">{copy.noResult}</p>
              ) : (
                <ul>
                  {results.map((champion) => (
                    <li key={champion.key}>
                      <Link href={`/${locale}/champions/${championSlug(champion)}`} onClick={close}>
                        {version ? (
                          <img className="icon" src={championIconUrl(version, champion.id)} alt="" width={28} height={28} />
                        ) : null}
                        {champion.name}
                      </Link>
                    </li>
                  ))}
                </ul>
              )}
            </section>
          ) : null}
          {!query.trim() ? <p className="muted">{copy.hint}</p> : null}
          <p>
            <button type="button" onClick={close}>
              {copy.close}
            </button>
          </p>
        </form>
      </dialog>
    </>
  );
}
