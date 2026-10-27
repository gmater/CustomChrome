import { mockIPC } from "@tauri-apps/api/mocks";
import { FN_INDEX, KEYS } from "../lib/layout";

/** Lets `npm run dev` render in a plain browser for UI work; never bundled into the app. */
export function installMockKeyboard() {
  document.body.style.background = "linear-gradient(135deg, #1d2030, #121318 60%, #1a1424)";
  let lighting = { effect: 0x0b, color: [255, 45, 85], rainbow: true, brightness: 5, speed: 3, direction: 0 };
  const factory: [number, number[]][] = KEYS.filter((k) => k.index !== FN_INDEX).map((k) => [k.index, [2, 0, 0x04 + (k.index % 90), 0]]);
  factory.push([FN_INDEX, [2, 0, 0xaf, 0]]);
  const defaults = () => {
    const t = Array.from({ length: 128 }, () => [0, 0, 0, 0]);
    for (const [i, e] of factory) t[i] = [...e];
    return t;
  };
  let keys = defaults();
  keys[48] = [2, 0xe0, 0, 0];
  const later = <T,>(v: T) => new Promise<T>((resolve) => setTimeout(() => resolve(v), 150));
  type Ev = { code: number; down: boolean; mouse: boolean; delay: number };
  const tap = (code: number): Ev[] => [
    { code, down: true, mouse: false, delay: 20 },
    { code, down: false, mouse: false, delay: 20 },
  ];
  let macros: Ev[][] = [[...tap(0x0b), ...tap(0x08), ...tap(0x0f), ...tap(0x0f), ...tap(0x12)]];
  localStorage.setItem("macro-names", JSON.stringify(["Type hello"]));
  keys[33] = [6, 0, 1, 2];
  mockIPC((cmd, args) => {
    switch (cmd) {
      case "read_keys":
        return keys;
      case "read_macros":
        return { macros, keys };
      case "macro_capacity":
        return 156;
      case "save_macro": {
        const { index, events } = args as { index: number; events: Ev[] };
        macros = macros.slice();
        macros[index] = events;
        return later({ macros, keys });
      }
      case "delete_macro": {
        const { index } = args as { index: number };
        macros = macros.filter((_, i) => i !== index);
        keys = keys.map((e, i) => (e[0] !== 6 ? e : e[1] === index ? factory.find(([f]) => f === i)![1] : e[1] > index ? [6, e[1] - 1, e[2], e[3]] : e));
        return later({ macros, keys });
      }
      case "factory_keys":
        return factory;
      case "assign_key": {
        const { index, entry } = args as { index: number; entry: number[] | null };
        keys = keys.map((e, i) => (i === index ? entry ?? factory.find(([f]) => f === i)![1] : e));
        return later(keys);
      }
      case "reset_keys":
        keys = defaults();
        return later(keys);
      case "detect_keyboard":
        return { name: "F99 MAX Dongle", connection: "dongle", supported: true, screenUpload: false };
      case "keyboard_info":
        return { firmware: "1.22", battery: 56, maxFrames: 104 };
      case "read_lighting":
        return lighting;
      case "write_lighting":
        lighting = (args as { lighting: typeof lighting }).lighting;
        return new Promise((resolve) => setTimeout(() => resolve(lighting), 150));
      default:
        return null;
    }
  });
}
