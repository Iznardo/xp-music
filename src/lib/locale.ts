// Language detection shared by the app and the website. No runes or DOM access,
// so Node can import it directly in tests.

export type Language = "en" | "es";

/** `es`, `es-ES`, `es_MX.UTF-8`… are Spanish; anything else is English. */
export function languageFromTag(tag: string | undefined | null): Language {
  return /^es([-_.@:]|$)/i.test((tag ?? "").trim()) ? "es" : "en";
}
