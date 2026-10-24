// node --test site/locale.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { languageFromTag } from "../src/lib/locale.ts";

test("Spanish locales map to es", () => {
  for (const tag of ["es", "es-ES", "es-419", "es_MX.UTF-8", "ES", " es-AR "]) assert.equal(languageFromTag(tag), "es", tag);
});

test("everything else maps to en", () => {
  for (const tag of ["en", "en-US", "fr-FR", "est", "et-EE", "ca-ES", "", undefined, null]) assert.equal(languageFromTag(tag), "en", String(tag));
});
