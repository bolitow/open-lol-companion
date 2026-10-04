import { errorMessage, type Locale, type SiteErrorCode } from "../lib/i18n";

export function ErrorNotice({ locale, code }: { locale: Locale; code: SiteErrorCode }) {
  return (
    <p className="notice" role="status">
      {errorMessage(locale, code)}
    </p>
  );
}
