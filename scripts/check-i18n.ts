import { validateCatalogs } from "./i18n-catalogs";

const root = new URL("../locales/", import.meta.url);
const english: unknown = await Bun.file(new URL("en.json", root)).json();
const german: unknown = await Bun.file(new URL("de.json", root)).json();
validateCatalogs(english, german);
console.log("I18N_CATALOGS_OK: matching keys, placeholders and plural forms");
