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
  const profiles: {
    auto: boolean;
    defaultProfile: string | null;
    profiles: { id: string; name: string }[];
    apps: { exe: string; label: string; profileId: string; path: string; icon: string }[];
    appliedId: string | null;
    appliedExe: string | null;
    armedId: string | null;
    hasMacros: boolean;
    macroNames: string[];
    restartSuggested: boolean;
  } = { auto: false, defaultProfile: null, profiles: [], apps: [], appliedId: null, appliedExe: null, armedId: null, hasMacros: false, macroNames: [], restartSuggested: false };
  mockIPC((cmd, args) => {
    switch (cmd) {
      case "read_keys":
        return keys;
      case "check_keyboard":
        return { issues: [{ kind: "damagedKey", key: 6, entry: [255, 255, 255, 255] }], macros: macros.length, remapped: [] };
      case "repair_keyboard":
        return later({ issues: [], macros: macros.length, remapped: [] });
      case "log_event":
        return null;
      case "save_report":
        return "D:\\Apps\\CustomChrome\\logs\\reports\\report-mock.txt";
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
        return { firmware: "1.22", battery: 56, charging: false, maxFrames: 104 };
      case "read_lighting":
        return lighting;
      case "read_board":
        return { sleepMinutes: 5, hour24: true };
      case "write_board":
        return later((args as { settings: { sleepMinutes: number; hour24: boolean } }).settings);
      case "sync_clock":
        return later(null);
      case "factory_reset":
        return later(null);
      case "open_logs":
        return null;
      case "profile_state":
        return profiles;
      case "save_profile": {
        const { name } = args as { name: string };
        const existing = profiles.profiles.find((p) => p.name.toLowerCase() === name.toLowerCase());
        if (existing) existing.name = name;
        else profiles.profiles.push({ id: `p${profiles.profiles.length + 1}`, name });
        return { ...profiles, profiles: profiles.profiles.slice() };
      }
      case "rename_profile": {
        const { id, name } = args as { id: string; name: string };
        profiles.profiles = profiles.profiles.map((p) => (p.id === id ? { ...p, name } : p));
        return { ...profiles };
      }
      case "delete_profile": {
        const { id } = args as { id: string };
        profiles.profiles = profiles.profiles.filter((p) => p.id !== id);
        profiles.apps = profiles.apps.filter((a) => a.profileId !== id);
        if (profiles.defaultProfile === id) profiles.defaultProfile = null;
        return { ...profiles };
      }
      case "apply_profile": {
        const { id } = args as { id: string };
        profiles.appliedId = id;
        return { ...profiles };
      }
      case "set_profile_auto": {
        profiles.auto = (args as { on: boolean }).on;
        return { ...profiles };
      }
      case "set_default_profile": {
        profiles.defaultProfile = (args as { id: string | null }).id;
        return { ...profiles };
      }
      case "assign_profile_app": {
        const link = args as { exe: string; label: string; path?: string; profileId: string };
        const next = { exe: link.exe, label: link.label, path: link.path ?? "", icon: "", profileId: link.profileId };
        profiles.apps = profiles.apps.filter((a) => a.exe !== link.exe).concat(next);
        return { ...profiles, apps: profiles.apps.slice() };
      }
      case "unassign_profile_app": {
        const { exe } = args as { exe: string };
        profiles.apps = profiles.apps.filter((a) => a.exe !== exe);
        return { ...profiles };
      }
      case "open_apps":
        return [
          { exe: "chrome.exe", title: "Google Chrome", path: "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe", icon: "" },
          { exe: "VALORANT.exe", title: "VALORANT", path: "C:\\Games\\VALORANT.exe", icon: "" },
        ];
      case "arm_program_capture":
        profiles.armedId = "*new";
        return { ...profiles };
      case "save_program": {
        const link = args as { exe: string; label: string; path?: string };
        const existing = profiles.apps.find((app) => app.exe === link.exe);
        if (!existing) {
          const id = `p${profiles.profiles.length + 1}`;
          profiles.profiles.push({ id, name: link.label });
          profiles.apps.push({ exe: link.exe, label: link.label, path: link.path ?? "", icon: "", profileId: id });
        }
        return { ...profiles, profiles: profiles.profiles.slice(), apps: profiles.apps.slice() };
      }
      case "cancel_profile_capture":
        profiles.armedId = null;
        return { ...profiles };
      case "write_lighting":
        lighting = (args as { lighting: typeof lighting }).lighting;
        return new Promise((resolve) => setTimeout(() => resolve(lighting), 150));
      default:
        return null;
    }
  });
}
