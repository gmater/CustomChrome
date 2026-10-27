export interface KeyCap {
  label: string;
  code?: string;
  x: number;
  y: number;
  w: number;
}

type Item = string | [label: string, width: number, code?: string] | { gap: number };

const letter = (l: string) => (/^[A-Z]$/.test(l) ? `Key${l}` : /^[0-9]$/.test(l) ? `Digit${l}` : undefined);

const ROWS: Item[][] = [
  ["Esc", { gap: 1 }, "F1", "F2", "F3", "F4", { gap: 0.25 }, "F5", "F6", "F7", "F8", { gap: 0.25 }, "F9", "F10", "F11", "F12"],
  [["`", 1, "Backquote"], "1", "2", "3", "4", "5", "6", "7", "8", "9", "0", ["-", 1, "Minus"], ["=", 1, "Equal"], ["Back", 2, "Backspace"], { gap: 0.25 }, ["Num", 1, "NumLock"], ["/", 1, "NumpadDivide"], ["*", 1, "NumpadMultiply"], ["-", 1, "NumpadSubtract"]],
  [["Tab", 1.5], "Q", "W", "E", "R", "T", "Y", "U", "I", "O", "P", ["[", 1, "BracketLeft"], ["]", 1, "BracketRight"], ["\\", 1.5, "Backslash"], { gap: 0.25 }, ["7", 1, "Numpad7"], ["8", 1, "Numpad8"], ["9", 1, "Numpad9"], ["+", 1, "NumpadAdd"]],
  [["Caps", 1.75, "CapsLock"], "A", "S", "D", "F", "G", "H", "J", "K", "L", [";", 1, "Semicolon"], ["'", 1, "Quote"], ["Enter", 2.25], { gap: 0.25 }, ["4", 1, "Numpad4"], ["5", 1, "Numpad5"], ["6", 1, "Numpad6"]],
  [["Shift", 2.25, "ShiftLeft"], "Z", "X", "C", "V", "B", "N", "M", [",", 1, "Comma"], [".", 1, "Period"], ["/", 1, "Slash"], ["Shift", 1.75, "ShiftRight"], ["↑", 1, "ArrowUp"], { gap: 0.25 }, ["1", 1, "Numpad1"], ["2", 1, "Numpad2"], ["3", 1, "Numpad3"], ["Ent", 1, "NumpadEnter"]],
  [["Ctrl", 1.25, "ControlLeft"], ["Win", 1.25, "MetaLeft"], ["Alt", 1.25, "AltLeft"], ["", 6.25, "Space"], ["Alt", 1, "AltRight"], ["Fn", 1], ["Ctrl", 1, "ControlRight"], ["←", 1, "ArrowLeft"], ["↓", 1, "ArrowDown"], ["→", 1, "ArrowRight"], { gap: 0.25 }, ["0", 1, "Numpad0"], [".", 1, "NumpadDecimal"]],
];

export const KEYBOARD_UNITS_W = 19.25;
export const KEYBOARD_UNITS_H = 6;

/** Stylised F99-style 96% layout; the light strip sits in the Esc–F1 gap and the screen in the top-right. */
export const KEYS: KeyCap[] = ROWS.flatMap((row, y) => {
  let x = 0;
  const keys: KeyCap[] = [];
  for (const item of row) {
    if (typeof item === "object" && !Array.isArray(item)) {
      x += item.gap;
      continue;
    }
    const [label, w, code] = typeof item === "string" ? [item, 1, undefined] : item;
    keys.push({ label, w, x, y, code: code ?? letter(label) ?? (/^F\d+$/.test(label) ? label : label === "Esc" ? "Escape" : label === "Tab" ? "Tab" : label === "Enter" ? "Enter" : undefined) });
    x += w;
  }
  return keys;
});

export const STRIP = { x: 1.2, y: 0.38, w: 0.6, h: 0.24 };
export const SCREEN = { x: 15.25, y: 0, w: 4, h: 1 };
