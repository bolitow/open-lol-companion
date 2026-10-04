"use client";

import { usePathname } from "next/navigation";
import type { MouseEvent } from "react";
import { localeHref } from "../lib/i18n";

interface Props {
  locale: string;
  target: string;
  label: string;
  text: string;
}

/** Même page dans l'autre langue ; les filtres de l'URL sont conservés au clic. */
export function LanguageSwitch({ locale, target, label, text }: Props) {
  const pathname = usePathname() ?? `/${locale}`;
  const href = localeHref(pathname, locale, target, "");
  const keepQuery = (event: MouseEvent<HTMLAnchorElement>) => {
    event.currentTarget.href = localeHref(pathname, locale, target, window.location.search);
  };
  return (
    <a href={href} hrefLang={target} lang={target} title={label} onClick={keepQuery}>
      {text}
    </a>
  );
}
