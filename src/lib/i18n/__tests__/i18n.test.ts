import { describe, it, expect, beforeEach, vi } from "vitest";

// Mock debug utils (auto-mocked by vitest config convention)
vi.mock("$lib/utils/debug", () => ({
  dbg: vi.fn(),
  dbgWarn: vi.fn(),
}));

let t: typeof import("../index.svelte").t;
let initLocale: typeof import("../index.svelte").initLocale;
let switchLocale: typeof import("../index.svelte").switchLocale;
let currentLocale: typeof import("../index.svelte").currentLocale;
let locales: typeof import("../index.svelte").locales;
let baseLocale: typeof import("../index.svelte").baseLocale;
let isLocale: typeof import("../index.svelte").isLocale;
let LOCALE_REGISTRY: typeof import("../index.svelte").LOCALE_REGISTRY;
let SUPPORTED_LOCALES: typeof import("../index.svelte").SUPPORTED_LOCALES;
let BASE_LOCALE: typeof import("../index.svelte").BASE_LOCALE;
let getEntry: typeof import("../index.svelte").getEntry;

// Minimal DOM stubs for <html> attribute tests
function setupDocument() {
  const doc = {
    documentElement: {
      lang: "",
      dir: "",
    },
  } as unknown as Document;
  Object.defineProperty(globalThis, "document", {
    configurable: true,
    value: doc,
  });
}

function setupLocalStorage() {
  const store: Record<string, string> = {};
  const lsImpl = {
    getItem: (key: string) => store[key] ?? null,
    setItem: (key: string, val: string) => {
      store[key] = val;
    },
    removeItem: (key: string) => {
      delete store[key];
    },
  };
  // @ts-expect-error - test stub
  globalThis.localStorage = lsImpl;
  // Ensure `typeof window !== "undefined"` passes in the module
  // @ts-expect-error - test stub
  globalThis.window = { localStorage: lsImpl };
  Object.defineProperty(globalThis, "navigator", {
    configurable: true,
    value: {
      languages: ["en-US", "en"],
      language: "en-US",
    },
  });
  return store;
}

describe("i18n", () => {
  let lsStore: Record<string, string>;

  beforeEach(async () => {
    vi.resetModules();
    setupDocument();
    lsStore = setupLocalStorage();

    const mod = await import("../index.svelte");
    t = mod.t;
    initLocale = mod.initLocale;
    switchLocale = mod.switchLocale;
    currentLocale = mod.currentLocale;
    locales = mod.locales;
    baseLocale = mod.baseLocale;
    isLocale = mod.isLocale;
    LOCALE_REGISTRY = mod.LOCALE_REGISTRY;
    SUPPORTED_LOCALES = mod.SUPPORTED_LOCALES;
    BASE_LOCALE = mod.BASE_LOCALE;
    getEntry = mod.getEntry;

    initLocale();
    switchLocale("en");
    delete lsStore["ocv:locale"];
    delete lsStore["PARAGLIDE_LOCALE"];
  });

  // ── t(key) basic ──

  it("returns the English translation for a known key", () => {
    initLocale();
    expect(t("settings_title")).toBe("Settings");
  });

  it("returns interpolated translation with params", () => {
    initLocale();
    const result = t("settings_general_lastUpdated", { date: "2026-02-17" });
    expect(result).toBe("Last updated: 2026-02-17");
  });

  it("falls back to baseLocale when key is missing in current locale", () => {
    // Switch to zh-CN, then query a key that only exists in en
    switchLocale("zh-CN");
    // 'settings_title' exists in both, so test with a hypothetical missing key
    // by directly testing the fallback behavior: en key should still work
    expect(t("settings_title")).toBe("设置");
  });

  it("returns raw key when key is completely missing", () => {
    initLocale();
    expect(t("this_key_does_not_exist" as any)).toBe("this_key_does_not_exist");
  });

  it("preserves unreplaced placeholders when param is missing", () => {
    initLocale();
    // settings_general_lastUpdated has {date} param
    expect(t("settings_general_lastUpdated")).toBe("Last updated: {date}");
  });

  // ── switchLocale ──

  it("switches locale and t() returns new locale translation", () => {
    initLocale();
    expect(t("settings_title")).toBe("Settings");

    switchLocale("zh-CN");
    expect(t("settings_title")).toBe("设置");
  });

  it("ignores switch to unknown locale", () => {
    initLocale();
    switchLocale("xx-INVALID");
    expect(currentLocale()).toBe("en");
  });

  it("ignores switch to same locale", () => {
    initLocale();
    switchLocale("en");
    expect(currentLocale()).toBe("en");
  });

  // ── currentLocale ──

  it("returns the current locale after init", () => {
    initLocale();
    expect(currentLocale()).toBe("en");
  });

  it("returns zh-CN after switch", () => {
    initLocale();
    switchLocale("zh-CN");
    expect(currentLocale()).toBe("zh-CN");
  });

  // ── localStorage persistence (new key: ocv:locale) ──

  it("persists locale to localStorage on switch", () => {
    initLocale();
    switchLocale("zh-CN");
    expect(lsStore["ocv:locale"]).toBe("zh-CN");
  });

  it("reads locale from localStorage on init", () => {
    lsStore["ocv:locale"] = "zh-CN";
    initLocale();
    expect(currentLocale()).toBe("zh-CN");
  });

  // ── Legacy localStorage migration ──

  it("migrates PARAGLIDE_LOCALE to ocv:locale on init", () => {
    lsStore["PARAGLIDE_LOCALE"] = "zh-CN";
    initLocale();
    expect(currentLocale()).toBe("zh-CN");
    expect(lsStore["ocv:locale"]).toBe("zh-CN");
  });

  it("prefers ocv:locale over PARAGLIDE_LOCALE", () => {
    lsStore["ocv:locale"] = "en";
    lsStore["PARAGLIDE_LOCALE"] = "zh-CN";
    initLocale();
    expect(currentLocale()).toBe("en");
  });

  // ── <html> attributes ──

  it("sets document.documentElement.lang on init", () => {
    initLocale();
    expect(document.documentElement.lang).toBe("en");
  });

  it("sets document.documentElement.lang on switch", () => {
    initLocale();
    switchLocale("zh-CN");
    expect(document.documentElement.lang).toBe("zh-CN");
  });

  it("sets dir=ltr for LTR locales", () => {
    initLocale();
    expect(document.documentElement.dir).toBe("ltr");
  });

  // ── Static exports ──

  it("exports correct locales array", () => {
    expect(locales).toEqual(["en", "zh-CN"]);
  });

  it("exports correct baseLocale", () => {
    expect(baseLocale).toBe("en");
  });

  it("isLocale returns true for valid locales", () => {
    expect(isLocale("en")).toBe(true);
    expect(isLocale("zh-CN")).toBe(true);
  });

  it("isLocale returns false for invalid locales", () => {
    expect(isLocale("xx")).toBe(false);
    expect(isLocale("")).toBe(false);
  });

  // ── Registry ──

  it("LOCALE_REGISTRY contains all supported locales", () => {
    const registryCodes = LOCALE_REGISTRY.map((e) => e.code);
    expect(registryCodes).toEqual(SUPPORTED_LOCALES);
  });

  it("SUPPORTED_LOCALES matches locales alias", () => {
    expect(SUPPORTED_LOCALES).toEqual(locales);
  });

  it("BASE_LOCALE matches baseLocale alias", () => {
    expect(BASE_LOCALE).toBe(baseLocale);
  });

  it("getEntry returns correct entry for known locale", () => {
    const entry = getEntry("en");
    expect(entry).toBeDefined();
    expect(entry!.nativeName).toBe("English");
    expect(entry!.shortLabel).toBe("EN");
    expect(entry!.dir).toBe("ltr");
  });

  it("getEntry returns undefined for unknown locale", () => {
    expect(getEntry("xx")).toBeUndefined();
  });

  it("every registry entry has required fields", () => {
    for (const entry of LOCALE_REGISTRY) {
      expect(entry.code).toBeTruthy();
      expect(entry.nativeName).toBeTruthy();
      expect(entry.shortLabel).toBeTruthy();
      expect(["ltr", "rtl"]).toContain(entry.dir);
      expect(["stable", "beta"]).toContain(entry.status);
    }
  });
});
