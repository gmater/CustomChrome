export interface KeyCap {
  label: string;
  /** Matrix index in the keyboard's key table. */
  index: number;
  /** `KeyboardEvent.code`, used to light keys you press in the preview. */
  code?: string;
  x: number;
  y: number;
  w: number;
}

type Item = [label: string, index: number, width?: number, code?: string] | number;

const k = (label: string, index: number, width = 1, code?: string): Item => [label, index, width, code];
const letters = (s: string, first: number) => [...s].map((l, i) => k(l, first + i, 1, `Key${l}`));
const digits = [..."1234567890"].map((d, i) => k(d, 17 + i, 1, `Digit${d}`));
const fkeys = (from: number) => [0, 1, 2, 3].map((n) => k(`F${from + n}`, from + n, 1, `F${from + n}`));

/** The F99 MAX's real layout, from the official app's matrix map; numbers are gaps in key units. */
const ROWS: Item[][] = [
  [k("Esc", 0, 1, "Escape"), 1, ...fkeys(1), 0.25, ...fkeys(5), 0.25, ...fkeys(9), 0.5, k("Del", 106, 1, "Delete")],
  [k("`", 16, 1, "Backquote"), ...digits, k("-", 27, 1, "Minus"), k("=", 28, 1, "Equal"), k("Back", 92, 2, "Backspace"), k("Home", 104, 1, "Home"), 0.25, k("Num", 29, 1, "NumLock"), k("/", 30, 1, "NumpadDivide"), k("*", 31, 1, "NumpadMultiply"), k("-", 109, 1, "NumpadSubtract")],
  [k("Tab", 32, 1.5, "Tab"), ...letters("QWERTYUIOP", 33), k("[", 43, 1, "BracketLeft"), k("]", 44, 1, "BracketRight"), k("\\", 60, 1.5, "Backslash"), k("PgUp", 105, 1, "PageUp"), 0.25, k("7", 45, 1, "Numpad7"), k("8", 46, 1, "Numpad8"), k("9", 47, 1, "Numpad9"), k("+", 110, 1, "NumpadAdd")],
  [k("Caps", 48, 1.75, "CapsLock"), ...letters("ASDFGHJKL", 49), k(";", 58, 1, "Semicolon"), k("'", 59, 1, "Quote"), k("Enter", 76, 2.25, "Enter"), k("PgDn", 108, 1, "PageDown"), 0.25, k("4", 61, 1, "Numpad4"), k("5", 62, 1, "Numpad5"), k("6", 63, 1, "Numpad6")],
  [k("Shift", 64, 2.25, "ShiftLeft"), ...letters("ZXCVBNM", 65), k(",", 72, 1, "Comma"), k(".", 73, 1, "Period"), k("/", 74, 1, "Slash"), k("Shift", 75, 1.75, "ShiftRight"), k("↑", 90, 1, "ArrowUp"), 1.25, k("1", 77, 1, "Numpad1"), k("2", 78, 1, "Numpad2"), k("3", 79, 1, "Numpad3"), k("Ent", 95, 1, "NumpadEnter")],
  [k("Ctrl", 80, 1.25, "ControlLeft"), k("Win", 81, 1.25, "MetaLeft"), k("Alt", 82, 1.25, "AltLeft"), k("", 83, 7, "Space"), k("Alt", 84, 1, "AltRight"), k("Fn", 85, 1), 0.25, k("←", 88, 1, "ArrowLeft"), k("↓", 89, 1, "ArrowDown"), k("→", 91, 1, "ArrowRight"), 0.25, k("0", 93, 2, "Numpad0"), k(".", 94, 1, "NumpadDecimal")],
];

export const KEYBOARD_UNITS_W = 20.25;
export const KEYBOARD_UNITS_H = 6;

export const KEYS: KeyCap[] = ROWS.flatMap((row, y) => {
  let x = 0;
  const keys: KeyCap[] = [];
  for (const item of row) {
    if (typeof item === "number") {
      x += item;
      continue;
    }
    const [label, index, w = 1, code] = item;
    keys.push({ label, index, w, x, y, code });
    x += w;
  }
  return keys;
});

export const FN_INDEX = 85;
export const STRIP = { x: 1.43, y: 0.1, w: 0.14, h: 0.8 };
export const SCREEN = { x: 16.25, y: 0, w: 4, h: 1 };
