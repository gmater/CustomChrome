export type KeyEntry = [number, number, number, number];

export interface Assignable {
  label: string;
  entry: KeyEntry;
}

export interface Group {
  name: string;
  items: Assignable[];
}

const key = (label: string, hid: number): Assignable => ({ label, entry: [2, 0, hid, 0] });
const mod = (label: string, hid: number): Assignable => ({ label, entry: [2, hid, 0, 0] });
const media = (label: string, usage: number): Assignable => ({ label, entry: [3, usage & 0xff, usage >> 8, 0] });
const mouse = (label: string, kind: number, value: number): Assignable => ({ label, entry: [1, kind, value, 0] });

const range = (labels: string, firstHid: number) => [...labels].map((l, i) => key(l, firstHid + i));

export const GROUPS: Group[] = [
  { name: "Letters", items: range("ABCDEFGHIJKLMNOPQRSTUVWXYZ", 0x04) },
  { name: "Numbers", items: [...range("123456789", 0x1e), key("0", 0x27)] },
  {
    name: "Function",
    items: [
      ...Array.from({ length: 12 }, (_, i) => key(`F${i + 1}`, 0x3a + i)),
      ...Array.from({ length: 12 }, (_, i) => key(`F${i + 13}`, 0x68 + i)),
    ],
  },
  {
    name: "Editing",
    items: [
      key("Esc", 0x29), key("Enter", 0x28), key("Backspace", 0x2a), key("Tab", 0x2b), key("Space", 0x2c),
      key("Caps Lock", 0x39), key("Insert", 0x49), key("Delete", 0x4c), key("Home", 0x4a), key("End", 0x4d),
      key("Page Up", 0x4b), key("Page Down", 0x4e), key("Print Screen", 0x46), key("Scroll Lock", 0x47),
      key("Pause", 0x48), key("Menu", 0x65),
    ],
  },
  { name: "Arrows", items: [key("↑", 0x52), key("↓", 0x51), key("←", 0x50), key("→", 0x4f)] },
  {
    name: "Symbols",
    items: [
      key("`", 0x35), key("-", 0x2d), key("=", 0x2e), key("[", 0x2f), key("]", 0x30), key("\\", 0x31),
      key(";", 0x33), key("'", 0x34), key(",", 0x36), key(".", 0x37), key("/", 0x38),
    ],
  },
  {
    name: "Modifiers",
    items: [
      mod("Left Ctrl", 0xe0), mod("Left Shift", 0xe1), mod("Left Alt", 0xe2), mod("Left Win", 0xe3),
      mod("Right Ctrl", 0xe4), mod("Right Shift", 0xe5), mod("Right Alt", 0xe6), mod("Right Win", 0xe7),
    ],
  },
  {
    name: "Numpad",
    items: [
      key("Num Lock", 0x53), key("Num /", 0x54), key("Num *", 0x55), key("Num -", 0x56), key("Num +", 0x57),
      key("Num Enter", 0x58), ...range("123456789", 0x59).map((a) => ({ ...a, label: `Num ${a.label}` })),
      key("Num 0", 0x62), key("Num .", 0x63),
    ],
  },
  {
    name: "Media",
    items: [
      media("Play / Pause", 0xcd), media("Stop", 0xb7), media("Previous", 0xb6), media("Next", 0xb5),
      media("Volume +", 0xe9), media("Volume −", 0xea), media("Mute", 0xe2), media("Media player", 0x183),
      media("Calculator", 0x192), media("Mail", 0x18a), media("This PC", 0x194), media("Browser home", 0x223),
      media("Browser back", 0x224), media("Browser forward", 0x225), media("Refresh", 0x227), media("Favourites", 0x22a),
    ],
  },
  {
    name: "Mouse",
    items: [
      mouse("Left click", 1, 1), mouse("Right click", 1, 2), mouse("Middle click", 1, 4),
      mouse("Back button", 1, 8), mouse("Forward button", 1, 16), mouse("Scroll up", 3, 1), mouse("Scroll down", 3, 255),
    ],
  },
];

const same = (a: KeyEntry, b: KeyEntry) => a.every((v, i) => v === b[i]);

export function describe(entry: KeyEntry): string {
  for (const g of GROUPS) {
    const hit = g.items.find((a) => same(a.entry, entry));
    if (hit) return hit.label;
  }
  if (entry[0] === 6) return `Macro ${entry[1] + 1}`;
  if (entry.every((v) => v === 0)) return "Default";
  return "Special";
}

export { same as sameEntry };
