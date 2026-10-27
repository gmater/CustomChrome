import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { KeyEntry } from "./keycodes";

export type Connection = "wired" | "dongle";

export interface KeyboardSummary {
  name: string;
  connection: Connection;
  supported: boolean;
  screenUpload: boolean;
}

export interface Lighting {
  effect: number;
  color: [number, number, number];
  rainbow: boolean;
  brightness: number;
  speed: number;
  direction: number;
}

export interface KeyboardInfo {
  firmware: string;
  battery: number;
  maxFrames: number;
}

export const detectKeyboard = () => invoke<KeyboardSummary | null>("detect_keyboard");

export const keyboardInfo = () => invoke<KeyboardInfo>("keyboard_info");

export const readKeys = () => invoke<KeyEntry[]>("read_keys");

export const factoryKeys = () => invoke<[number, KeyEntry][]>("factory_keys");

export const assignKey = (index: number, entry: KeyEntry | null) => invoke<KeyEntry[]>("assign_key", { index, entry });

export const resetKeys = () => invoke<KeyEntry[]>("reset_keys");

export const readLighting = () => invoke<Lighting>("read_lighting");

export const writeLighting = (lighting: Lighting) => invoke<Lighting>("write_lighting", { lighting });

export async function uploadScreen(
  rgba: Uint8Array,
  delaysCs: number[],
  onProgress: (percent: number) => void,
): Promise<number> {
  const unlisten = await listen<number>("upload-progress", (e) => onProgress(e.payload));
  try {
    return await invoke<number>("upload_screen", rgba, {
      headers: { "x-delays": delaysCs.map((d) => Math.max(1, Math.round(d))).join(",") },
    });
  } finally {
    unlisten();
  }
}

export function errorText(err: unknown): string {
  return typeof err === "string" ? err : err instanceof Error ? err.message : "Something went wrong.";
}
