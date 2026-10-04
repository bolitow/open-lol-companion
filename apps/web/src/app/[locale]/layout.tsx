import type { Metadata } from "next";
import { notFound } from "next/navigation";
import type { ReactNode } from "react";
import { LanguageSwitch } from "../../components/LanguageSwitch";
import { SearchDialog } from "../../components/SearchDialog";
import { dictionary, isLocale, LOCALES } from "../../lib/i18n";
import { loadStatic } from "../../lib/server";
import "../globals.css";

export const dynamicParams = false;

export function generateStaticParams() {
  return LOCALES.map((locale) => ({ locale }));
}

export async function generateMetadata({ params }: { params: Promise<{ locale: string }> }): Promise<Metadata> {
  const { locale } = await params;
  if (!isLocale(locale)) return {};
  const t = dictionary(locale).site;
  return { title: { default: t.name, template: `%s · ${t.name}` }, description: t.description };
}

export default async function LocaleLayout({ children, params }: { children: ReactNode; params: Promise<{ locale: string }> }) {
  const { locale } = await params;
  if (!isLocale(locale)) notFound();
  const t = dictionary(locale);
  const statics = await loadStatic(locale);
  const champions = statics ? [...statics.champions.values()].sort((a, b) => a.name.localeCompare(b.name, locale)) : [];
  const other = locale === "fr" ? "en" : "fr";
  return (
    <html lang={locale}>
      <body>
        <header className="header">
          <a className="brand" href={`/${locale}/tierlist`}>
            {t.site.name}
          </a>
          <nav aria-label={t.site.navigation}>
            <a href={`/${locale}/tierlist`}>{t.site.tierlist}</a>
          </nav>
          <SearchDialog locale={locale} version={statics?.version ?? null} champions={champions} copy={t.search} />
          <LanguageSwitch locale={locale} target={other} label={t.site.otherLanguageLabel} text={t.site.otherLanguage} />
        </header>
        <main>{children}</main>
        <footer className="footer">
          <p>{t.site.sources}</p>
          <p>{t.site.legal}</p>
        </footer>
      </body>
    </html>
  );
}
