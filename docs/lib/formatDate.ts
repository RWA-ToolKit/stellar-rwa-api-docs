/**
 * Default locale for user-visible dates in the docs site.
 *
 * The docs are currently published in English only, so dates default to
 * `en-US` (e.g. "November 21, 2026"). Components that render dates accept an
 * optional `locale` prop so a future i18n layer can pass the active locale
 * without changing call sites that rely on this default.
 */
export const DEFAULT_LOCALE = "en-US";

const ISO_DATE_ONLY = /^\d{4}-\d{2}-\d{2}$/;

/**
 * Format a date with `Intl.DateTimeFormat` for the given locale.
 *
 * Formatting is pinned to `timeZone: "UTC"` so that ISO date-only strings
 * (`YYYY-MM-DD`, interpreted as UTC midnight) never shift by a day in
 * negative-offset time zones, and so server and client renders produce the
 * same text (no hydration mismatch). Callers may override any option.
 */
export function formatDate(
  date: Date | string,
  locale: string = DEFAULT_LOCALE,
  options?: Intl.DateTimeFormatOptions,
): string {
  const value =
    typeof date === "string"
      ? new Date(ISO_DATE_ONLY.test(date) ? `${date}T00:00:00Z` : date)
      : date;
  return new Intl.DateTimeFormat(locale, {
    year: "numeric",
    month: "long",
    day: "numeric",
    timeZone: "UTC",
    ...options,
  }).format(value);
}
