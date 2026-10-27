import { useRef } from "react";

type Rgb = [number, number, number];

interface Props {
  value: Rgb;
  onChange: (rgb: Rgb) => void;
  size?: number;
}

function hsvToRgb(h: number, s: number, v: number): Rgb {
  const f = (n: number) => {
    const k = (n + h / 60) % 6;
    return Math.round(255 * (v - v * s * Math.max(0, Math.min(k, 4 - k, 1))));
  };
  return [f(5), f(3), f(1)];
}

function rgbToHs([r, g, b]: Rgb): { h: number; s: number } {
  const max = Math.max(r, g, b);
  const d = max - Math.min(r, g, b);
  if (max === 0 || d === 0) return { h: 0, s: 0 };
  let h = max === r ? (g - b) / d : max === g ? 2 + (b - r) / d : 4 + (r - g) / d;
  h = (h * 60 + 360) % 360;
  return { h, s: d / max };
}

/** Hue around the rim, saturation toward the centre; the separate brightness slider covers value. */
export default function ColorWheel({ value, onChange, size = 200 }: Props) {
  const ref = useRef<HTMLDivElement>(null);
  const radius = size / 2;
  const { h, s } = rgbToHs(value);
  const angle = (h * Math.PI) / 180;
  const markerX = radius + Math.sin(angle) * s * radius;
  const markerY = radius - Math.cos(angle) * s * radius;

  const pick = (clientX: number, clientY: number) => {
    const rect = ref.current!.getBoundingClientRect();
    const dx = clientX - rect.left - radius;
    const dy = clientY - rect.top - radius;
    const hue = ((Math.atan2(dx, -dy) * 180) / Math.PI + 360) % 360;
    const sat = Math.min(1, Math.hypot(dx, dy) / radius);
    onChange(hsvToRgb(hue, sat, 1));
  };

  return (
    <div
      ref={ref}
      className="color-wheel"
      style={{ width: size, height: size }}
      onPointerDown={(e) => {
        e.currentTarget.setPointerCapture(e.pointerId);
        pick(e.clientX, e.clientY);
      }}
      onPointerMove={(e) => {
        if (e.currentTarget.hasPointerCapture(e.pointerId)) pick(e.clientX, e.clientY);
      }}
    >
      <span
        className="color-wheel-marker"
        style={{ left: markerX, top: markerY, background: `rgb(${value.join(",")})` }}
      />
    </div>
  );
}
