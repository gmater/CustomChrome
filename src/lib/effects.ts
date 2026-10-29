export type DirectionAxis = "horizontal" | "vertical";

/** How the on-screen preview animates an effect, matching the F99 MAX firmware patterns. */
export type Motion =
  | "static"
  | "on"
  | "quench"
  | "twinkle"
  | "snow"
  | "blossom"
  | "breathe"
  | "spectrum"
  | "fountain"
  | "cross"
  | "wave"
  | "peaks"
  | "burst"
  | "split"
  | "ripple"
  | "flow"
  | "ridges"
  | "rain"
  | "bounce";

export interface Effect {
  id: number;
  name: string;
  usesColor: boolean;
  hasSpeed: boolean;
  direction?: DirectionAxis;
  motion: Motion;
}

export const EFFECT_OFF = 0;
export const LEVEL_MAX = 6;

export const EFFECTS: Effect[] = [
  { id: 0x01, name: "Static", usesColor: true, hasSpeed: false, motion: "static" },
  { id: 0x02, name: "Single key on", usesColor: true, hasSpeed: true, motion: "on" },
  { id: 0x03, name: "Single key off", usesColor: true, hasSpeed: true, motion: "quench" },
  { id: 0x04, name: "Starry sky", usesColor: true, hasSpeed: true, motion: "twinkle" },
  { id: 0x05, name: "Snowfall", usesColor: true, hasSpeed: true, motion: "snow" },
  { id: 0x06, name: "Blossom", usesColor: false, hasSpeed: true, motion: "blossom" },
  { id: 0x07, name: "Breathing", usesColor: true, hasSpeed: true, motion: "breathe" },
  { id: 0x08, name: "Spectrum cycle", usesColor: false, hasSpeed: true, motion: "spectrum" },
  { id: 0x09, name: "Colour fountain", usesColor: true, hasSpeed: true, motion: "fountain" },
  { id: 0x0a, name: "Colour crossing", usesColor: true, hasSpeed: true, direction: "vertical", motion: "cross" },
  { id: 0x0b, name: "Wave", usesColor: true, hasSpeed: true, direction: "horizontal", motion: "wave" },
  { id: 0x0c, name: "Turning peaks", usesColor: true, hasSpeed: true, direction: "horizontal", motion: "peaks" },
  { id: 0x0d, name: "One touch fire", usesColor: true, hasSpeed: true, motion: "burst" },
  { id: 0x0e, name: "Two birds", usesColor: true, hasSpeed: true, motion: "split" },
  { id: 0x0f, name: "Ripples", usesColor: true, hasSpeed: true, motion: "ripple" },
  { id: 0x10, name: "Endless flow", usesColor: true, hasSpeed: true, direction: "horizontal", motion: "flow" },
  { id: 0x11, name: "Layered mountains", usesColor: true, hasSpeed: true, motion: "ridges" },
  { id: 0x12, name: "Gentle rain", usesColor: true, hasSpeed: true, direction: "horizontal", motion: "rain" },
  { id: 0x13, name: "Back and forth", usesColor: true, hasSpeed: true, motion: "bounce" },
];

export const SWATCHES = [
  "#FFFFFF",
  "#FF2D55",
  "#FF3B30",
  "#FF9500",
  "#FFD60A",
  "#34C759",
  "#00E5FF",
  "#0A84FF",
  "#5E5CE6",
  "#BF5AF2",
];

export const durationFor = (speed: number) => `${(7.2 / Math.max(1, speed)).toFixed(2)}s`;

export const toHex = ([r, g, b]: [number, number, number]) =>
  "#" + [r, g, b].map((v) => v.toString(16).padStart(2, "0")).join("").toUpperCase();

export const fromHex = (hex: string): [number, number, number] => [
  parseInt(hex.slice(1, 3), 16),
  parseInt(hex.slice(3, 5), 16),
  parseInt(hex.slice(5, 7), 16),
];
