import { beforeEach } from "bun:test";
import { locale } from "../../apps/desktop/src/lib/i18n";

// Existing regression expectations are German; localization tests choose explicitly.
beforeEach(() => locale.set("de"));
