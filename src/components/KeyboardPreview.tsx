import { useEffect, useMemo, useRef, type CSSProperties } from "react";
import { durationFor, toHex, type Effect } from "../lib/effects";
import { KEYBOARD_UNITS_H, KEYBOARD_UNITS_W, KEYS, SCREEN, STRIP } from "../lib/layout";
import type { Lighting } from "../lib/keyboard";

interface Props {
  lighting: Lighting;
  effect: Effect | undefined;
  on: boolean;
}

const PRESS = new Set(["on", "quench", "burst", "split", "ripple"]);

const pct = (v: number, of: number) => `${(v / of) * 100}%`;
const seeded = (i: number) => {
  const s = Math.sin(i * 12.9898) * 43758.5453;
  return s - Math.floor(s);
};

export function fxClasses(effect: Effect | undefined, lighting: Lighting, on: boolean) {
  const motion = on ? (effect?.motion ?? "static") : "off";
  const rainbow = lighting.rainbow || (effect ? !effect.usesColor : false);
  return [
    "fx",
    `fx-${motion}`,
    rainbow ? "rainbow" : "",
    effect?.direction === "vertical" ? "axis-y" : "",
    effect?.direction && lighting.direction === 1 ? "reverse" : "",
  ].join(" ");
}

export function fxStyle(lighting: Lighting): CSSProperties {
  return {
    "--c": toHex(lighting.color),
    "--dur": durationFor(lighting.speed),
    "--bri": String(0.35 + (lighting.brightness / 6) * 0.65),
  } as CSSProperties;
}

export default function KeyboardPreview({ lighting, effect, on }: Props) {
  const ref = useRef<HTMLDivElement>(null);
  const motion = on ? effect?.motion : undefined;

  const keys = useMemo(
    () =>
      KEYS.map((k, i) => {
        const cx = (k.x + k.w / 2) / KEYBOARD_UNITS_W;
        const cy = (k.y + 0.5) / KEYBOARD_UNITS_H;
        return {
          ...k,
          nx: cx.toFixed(3),
          ny: cy.toFixed(3),
          style: {
            left: pct(k.x, KEYBOARD_UNITS_W),
            top: pct(k.y, KEYBOARD_UNITS_H),
            width: pct(k.w, KEYBOARD_UNITS_W),
            height: pct(1, KEYBOARD_UNITS_H),
            "--x": cx.toFixed(3),
            "--y": cy.toFixed(3),
            "--r": seeded(i + 1).toFixed(3),
            "--d": Math.min(1, Math.hypot(cx - 0.5, (cy - 0.5) * 0.6) * 1.9).toFixed(3),
          } as CSSProperties,
        };
      }),
    [],
  );

  useEffect(() => {
    if (!motion || !PRESS.has(motion)) return;
    const root = ref.current!;
    const pending = new Set<number>();
    const flash = (el: Element, delay = 0) => {
      const go = () => {
        el.classList.remove("hit");
        void (el as HTMLElement).offsetWidth;
        el.classList.add("hit");
      };
      if (delay <= 0) go();
      else pending.add(window.setTimeout(go, delay));
    };
    const play = (origin: Element) => {
      const ox = Number((origin as HTMLElement).dataset.x);
      const oy = Number((origin as HTMLElement).dataset.y);
      const all = [...root.querySelectorAll<HTMLElement>(".key")];
      const place = (el: HTMLElement) => {
        const dx = Number(el.dataset.x) - ox;
        const dy = Number(el.dataset.y) - oy;
        return { dx, dy, d: Math.hypot(dx, dy * 1.5) };
      };
      const travel = 7200 / Math.max(1, lighting.speed);
      if (motion === "on" || motion === "quench") flash(origin);
      else if (motion === "burst") all.forEach((el) => {
        const { dx, dy } = place(el);
        if (Math.abs(dy) < 0.08) flash(el, Math.abs(dx) * travel);
      });
      else if (motion === "split") all.forEach((el) => {
        const { dx, dy } = place(el);
        if (Math.abs(dy) < 0.09) flash(el, Math.abs(dx) * 420);
      });
      else all.forEach((el) => flash(el, place(el).d * 380));
    };
    const onKey = (e: KeyboardEvent) => {
      const el = root.querySelector(`[data-code="${CSS.escape(e.code)}"]`);
      if (el) play(el);
    };
    const all = root.querySelectorAll(".key");
    const timer = window.setInterval(() => play(all[Math.floor(Math.random() * all.length)]), 7000 / (lighting.speed + 4));
    window.addEventListener("keydown", onKey);
    return () => {
      clearInterval(timer);
      pending.forEach((id) => clearTimeout(id));
      window.removeEventListener("keydown", onKey);
    };
  }, [motion, lighting.speed]);

  useEffect(() => {
    if (motion !== "twinkle") return;
    const root = ref.current!;
    const gap = 7200 / Math.max(1, lighting.speed) / 5;
    root.style.setProperty("--star", `${gap}ms`);
    const all = [...root.querySelectorAll<HTMLElement>(".key")];
    let current: HTMLElement | null = null;
    const hop = () => {
      if (!all.length) return;
      let next = all[Math.floor(Math.random() * all.length)];
      if (all.length > 1) {
        while (next === current) next = all[Math.floor(Math.random() * all.length)];
      }
      if (current) current.classList.remove("star");
      current = next;
      void current.offsetWidth;
      current.classList.add("star");
    };
    hop();
    const timer = window.setInterval(hop, gap);
    return () => {
      clearInterval(timer);
      if (current) current.classList.remove("star");
    };
  }, [motion, lighting.speed]);

  return (
    <div ref={ref} className={`kb ${fxClasses(effect, lighting, on)}`} style={fxStyle(lighting)}>
      <div className="kb-plate">
        {keys.map((k, i) => (
          <div key={i} className="key" data-code={k.code} data-x={k.nx} data-y={k.ny} style={k.style}>
            <span className="cap">
              <span className="led-wrap">
                <span className="led" />
              </span>
              <span className="legend">{k.label}</span>
            </span>
          </div>
        ))}
        <div
          className="kb-strip"
          title="Light strip: controlled on the keyboard with Fn + Right Shift"
          style={{
            left: pct(STRIP.x, KEYBOARD_UNITS_W),
            top: pct(STRIP.y, KEYBOARD_UNITS_H),
            width: pct(STRIP.w, KEYBOARD_UNITS_W),
            height: pct(STRIP.h, KEYBOARD_UNITS_H),
          }}
        />
        <div
          className="kb-screen"
          style={{
            left: pct(SCREEN.x, KEYBOARD_UNITS_W),
            top: pct(SCREEN.y, KEYBOARD_UNITS_H),
            width: pct(SCREEN.w, KEYBOARD_UNITS_W),
            height: pct(SCREEN.h, KEYBOARD_UNITS_H),
          }}
        >
          <span>F99 MAX</span>
        </div>
      </div>
    </div>
  );
}

export function MiniPreview({ effect, lighting }: { effect: Effect; lighting: Lighting }) {
  return (
    <span className={`mini ${fxClasses(effect, lighting, true)}`} style={fxStyle({ ...lighting, speed: 3, brightness: 6 })}>
      {Array.from({ length: 7 }, (_, i) => (
        <span
          key={i}
          className="key"
          style={{ "--x": (i / 6).toFixed(3), "--y": (i / 6).toFixed(3), "--r": seeded(i + 40).toFixed(3), "--d": (Math.abs(i - 3) / 3).toFixed(3) } as CSSProperties}
        >
          <span className="led-wrap">
            <span className="led" />
          </span>
        </span>
      ))}
    </span>
  );
}
