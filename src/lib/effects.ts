export type DirectionAxis = "horizontal" | "vertical";

export interface Effect {
  id: number;
  name: string;
  usesColor: boolean;
  hasSpeed: boolean;
  direction?: DirectionAxis;
}

export const EFFECT_OFF = 0;
export const LEVEL_MAX = 6;

export const EFFECTS: Effect[] = [
  { id: 0x01, name: "Static", usesColor: true, hasSpeed: false },
  { id: 0x02, name: "Single key on", usesColor: true, hasSpeed: true },
  { id: 0x03, name: "Single key off", usesColor: true, hasSpeed: true },
  { id: 0x04, name: "Starry sky", usesColor: true, hasSpeed: true },
  { id: 0x05, name: "Snowfall", usesColor: true, hasSpeed: true },
  { id: 0x06, name: "Blossom", usesColor: false, hasSpeed: true },
  { id: 0x07, name: "Breathing", usesColor: true, hasSpeed: true },
  { id: 0x08, name: "Spectrum cycle", usesColor: false, hasSpeed: true },
  { id: 0x09, name: "Colour fountain", usesColor: true, hasSpeed: true },
  { id: 0x0a, name: "Colour crossing", usesColor: true, hasSpeed: true, direction: "vertical" },
  { id: 0x0b, name: "Wave", usesColor: true, hasSpeed: true, direction: "horizontal" },
  { id: 0x0c, name: "Turning peaks", usesColor: true, hasSpeed: true, direction: "horizontal" },
  { id: 0x0d, name: "One touch fire", usesColor: true, hasSpeed: true },
  { id: 0x0e, name: "Two birds", usesColor: true, hasSpeed: true },
  { id: 0x0f, name: "Ripples", usesColor: true, hasSpeed: true },
  { id: 0x10, name: "Endless flow", usesColor: true, hasSpeed: true, direction: "horizontal" },
  { id: 0x11, name: "Layered mountains", usesColor: true, hasSpeed: true },
  { id: 0x12, name: "Gentle rain", usesColor: true, hasSpeed: true, direction: "horizontal" },
  { id: 0x13, name: "Back and forth", usesColor: true, hasSpeed: true },
];

export const SWATCHES = ["#FFFFFF", "#FF0000", "#FF00FF", "#0000FF", "#00FFFF", "#00FF00", "#FFFF00", "#FF7A00"];

export const toHex = ([r, g, b]: [number, number, number]) =>
  "#" + [r, g, b].map((v) => v.toString(16).padStart(2, "0")).join("").toUpperCase();

export const fromHex = (hex: string): [number, number, number] => [
  parseInt(hex.slice(1, 3), 16),
  parseInt(hex.slice(3, 5), 16),
  parseInt(hex.slice(5, 7), 16),
];
