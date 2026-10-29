export interface AppPrefs {
  rememberPage: boolean;
  accent: string;
  /** When on, buttons and highlights use the keyboard's key colour. */
  matchKeys: boolean;
  /** How the sidebar battery icon shows the percentage, as on an Android status bar. */
  batteryStyle: "hidden" | "beside" | "inside";
  macroReminder: boolean;
}

export const PREFS_KEY = "app-prefs";
export const LAST_PAGE_KEY = "last-page";

export const DEFAULT_PREFS: AppPrefs = {
  rememberPage: true,
  accent: "#8b7cff",
  matchKeys: false,
  batteryStyle: "beside",
  macroReminder: true,
};

const KEY_COLOR = "lighting-color";
const RAINBOW_ACCENT = "#8b7cff";

export interface KeyLook {
  color: string;
  rainbow: boolean;
}

function hexByte(n: number) {
  return Math.max(0, Math.min(255, Math.round(n))).toString(16).padStart(2, "0");
}

/** The colour last sent to the keys. Rainbow lighting has no single colour. */
export function loadKeyLook(): KeyLook {
  try {
    const saved = JSON.parse(localStorage.getItem(KEY_COLOR) ?? "null");
    const color = saved?.color;
    if (Array.isArray(color) && color.length === 3 && color.every((n) => typeof n === "number")) {
      return { color: `#${hexByte(color[0])}${hexByte(color[1])}${hexByte(color[2])}`, rainbow: saved.rainbow === true };
    }
  } catch {
    /* keep the default */
  }
  return { color: RAINBOW_ACCENT, rainbow: false };
}

export function accentColor(prefs: AppPrefs, look: KeyLook = loadKeyLook()) {
  if (!prefs.matchKeys) return prefs.accent;
  return look.rainbow ? RAINBOW_ACCENT : look.color;
}

const ACCENTS = ["#8b7cff", "#5aa8ff", "#3ddc97", "#ffb547", "#ff6b8a", "#f2f4f8"];

export const ACCENT_CHOICES = ACCENTS;

export function loadPrefs(): AppPrefs {
  try {
    const raw = JSON.parse(localStorage.getItem(PREFS_KEY) ?? "null");
    if (!raw || typeof raw !== "object") return { ...DEFAULT_PREFS };
    return {
      rememberPage: typeof raw.rememberPage === "boolean" ? raw.rememberPage : DEFAULT_PREFS.rememberPage,
      accent: typeof raw.accent === "string" && /^#[0-9a-fA-F]{6}$/.test(raw.accent) ? raw.accent : DEFAULT_PREFS.accent,
      matchKeys: raw.matchKeys === true,
      batteryStyle: raw.batteryStyle === "hidden" || raw.batteryStyle === "inside" ? raw.batteryStyle : "beside",
      macroReminder: typeof raw.macroReminder === "boolean" ? raw.macroReminder : DEFAULT_PREFS.macroReminder,
    };
  } catch {
    return { ...DEFAULT_PREFS };
  }
}

export function storePrefs(prefs: AppPrefs) {
  localStorage.setItem(PREFS_KEY, JSON.stringify(prefs));
}

export function applyAccent(accent: string) {
  document.documentElement.style.setProperty("--live", accent);
}
