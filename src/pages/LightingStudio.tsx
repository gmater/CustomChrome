import { useCallback, useEffect, useRef, useState } from "react";
import ColorWheel from "../components/ColorWheel";
import { EFFECT_OFF, EFFECTS, fromHex, LEVEL_MAX, SWATCHES, toHex } from "../lib/effects";
import { errorText, readLighting, writeLighting, type KeyboardSummary, type Lighting } from "../lib/keyboard";

interface Props {
  keyboard: KeyboardSummary | null;
}

type Save = "idle" | "saving" | "saved" | { error: string };

const WRITE_DELAY_MS = 250;
/** The firmware applies colour writes but reads back stale colour bytes, so the last colour sent is kept here. */
const COLOR_KEY = "lighting-color";

function rememberedColor(): Pick<Lighting, "color" | "rainbow"> | null {
  try {
    const saved = JSON.parse(localStorage.getItem(COLOR_KEY) ?? "null");
    return Array.isArray(saved?.color) && saved.color.length === 3 && typeof saved.rainbow === "boolean" ? saved : null;
  } catch {
    return null;
  }
}

export default function LightingStudio({ keyboard }: Props) {
  const [lighting, setLighting] = useState<Lighting | null>(null);
  const [loadError, setLoadError] = useState("");
  const [save, setSave] = useState<Save>("idle");
  const lastEffect = useRef(0x01);
  const latest = useRef<Lighting | null>(null);
  const timer = useRef(0);
  const writing = useRef(false);
  const dirty = useRef(false);

  const ready = !!keyboard?.supported;
  const keyboardKey = ready ? `${keyboard!.connection}:${keyboard!.name}` : "";

  useEffect(() => {
    setLighting(null);
    setLoadError("");
    setSave("idle");
    if (!keyboardKey) return;
    let alive = true;
    readLighting()
      .then((read) => {
        if (!alive) return;
        const l = { ...read, ...rememberedColor() };
        if (l.effect !== EFFECT_OFF) lastEffect.current = l.effect;
        latest.current = l;
        setLighting(l);
      })
      .catch((err) => alive && setLoadError(errorText(err)));
    return () => {
      alive = false;
    };
  }, [keyboardKey]);

  useEffect(() => () => clearTimeout(timer.current), []);

  const flush = useCallback(async () => {
    if (writing.current) {
      dirty.current = true;
      return;
    }
    writing.current = true;
    setSave("saving");
    try {
      do {
        dirty.current = false;
        const sent = latest.current!;
        await writeLighting(sent);
        localStorage.setItem(COLOR_KEY, JSON.stringify({ color: sent.color, rainbow: sent.rainbow }));
      } while (dirty.current);
      setSave("saved");
    } catch (err) {
      setSave({ error: errorText(err) });
    } finally {
      writing.current = false;
    }
  }, []);

  const update = (patch: Partial<Lighting>) => {
    if (!lighting) return;
    const next = { ...lighting, ...patch };
    if (next.effect !== EFFECT_OFF) lastEffect.current = next.effect;
    latest.current = next;
    setLighting(next);
    clearTimeout(timer.current);
    timer.current = window.setTimeout(flush, WRITE_DELAY_MS);
  };

  const header = (
    <header className="page-header">
      <div>
        <h1>Lighting</h1>
        <p>Pick an effect and colour. Changes are saved to the keyboard as you make them.</p>
      </div>
    </header>
  );

  if (!keyboard) {
    return (
      <section className="page">
        {header}
        <div className="banner">Connect the keyboard with its USB cable or wireless dongle.</div>
      </section>
    );
  }
  if (!keyboard.supported) {
    return (
      <section className="page">
        {header}
        <div className="banner warn">"{keyboard.name}" isn't an AULA F99 MAX, so CustomChrome won't change its lighting.</div>
      </section>
    );
  }
  if (!lighting) {
    return (
      <section className="page">
        {header}
        {loadError ? (
          <div className="banner warn">Couldn't read the keyboard's lighting: {loadError}</div>
        ) : (
          <div className="banner">Reading the keyboard's lighting…</div>
        )}
      </section>
    );
  }

  const on = lighting.effect !== EFFECT_OFF;
  const effect = EFFECTS.find((e) => e.id === (on ? lighting.effect : lastEffect.current));
  const hex = toHex(lighting.color);

  return (
    <section className="page">
      {header}

      <div className="lighting">
        <div className="panel effects-panel">
          <div className="panel-title">
            <label>Effect</label>
            <button
              className={`switch ${on ? "on" : ""}`}
              role="switch"
              aria-checked={on}
              onClick={() => update({ effect: on ? EFFECT_OFF : lastEffect.current })}
            >
              <span />
              {on ? "On" : "Off"}
            </button>
          </div>
          <div className={`effect-grid ${on ? "" : "dimmed"}`}>
            {EFFECTS.map((e) => (
              <button
                key={e.id}
                className={`effect ${effect?.id === e.id ? "on" : ""}`}
                onClick={() => update({ effect: e.id, direction: e.direction ? lighting.direction : 0 })}
              >
                <span
                  className="effect-swatch"
                  style={{
                    background:
                      e.usesColor && !lighting.rainbow
                        ? hex
                        : "conic-gradient(#ff4d4d, #ffd24d, #4dff88, #4dd2ff, #9b4dff, #ff4d4d)",
                  }}
                />
                {e.name}
              </button>
            ))}
          </div>
        </div>

        <div className="panel">
          {effect?.usesColor && (
            <div className="field">
              <label>Colour</label>
              <div className={`wheel-wrap ${lighting.rainbow ? "dimmed" : ""}`}>
                <ColorWheel value={lighting.color} onChange={(color) => update({ color, rainbow: false })} />
              </div>
              <div className="swatches">
                {SWATCHES.map((s) => (
                  <button
                    key={s}
                    className={`swatch ${!lighting.rainbow && hex === s ? "on" : ""}`}
                    style={{ background: s }}
                    aria-label={s}
                    onClick={() => update({ color: fromHex(s), rainbow: false })}
                  />
                ))}
              </div>
              <div className="color-row">
                <span className="color-chip" style={{ background: hex }} />
                <input
                  key={hex}
                  className="hex-input"
                  defaultValue={hex}
                  maxLength={7}
                  spellCheck={false}
                  onChange={(e) => {
                    const v = e.target.value.trim();
                    if (/^#?[0-9a-fA-F]{6}$/.test(v)) update({ color: fromHex(v.startsWith("#") ? v : `#${v}`), rainbow: false });
                  }}
                />
              </div>
              <label className="check">
                <input type="checkbox" checked={lighting.rainbow} onChange={(e) => update({ rainbow: e.target.checked })} />
                Rainbow colours
              </label>
            </div>
          )}

          <div className="field">
            <label>
              Brightness <b>{lighting.brightness}</b>
            </label>
            <input
              type="range"
              min={1}
              max={LEVEL_MAX}
              value={lighting.brightness}
              onChange={(e) => update({ brightness: Number(e.target.value) })}
            />
          </div>

          {effect?.hasSpeed && (
            <div className="field">
              <label>
                Speed <b>{lighting.speed}</b>
              </label>
              <input
                type="range"
                min={1}
                max={LEVEL_MAX}
                value={lighting.speed}
                onChange={(e) => update({ speed: Number(e.target.value) })}
              />
            </div>
          )}

          {effect?.direction && (
            <div className="field">
              <label>Direction</label>
              <div className="segmented two">
                <button className={lighting.direction === 1 ? "on" : ""} onClick={() => update({ direction: 1 })}>
                  {effect.direction === "horizontal" ? "← Left" : "↑ Up"}
                </button>
                <button className={lighting.direction === 0 ? "on" : ""} onClick={() => update({ direction: 0 })}>
                  {effect.direction === "horizontal" ? "Right →" : "Down ↓"}
                </button>
              </div>
            </div>
          )}

          <p className={`status ${typeof save === "object" ? "err" : save === "saved" ? "ok" : ""}`}>
            {typeof save === "object"
              ? save.error
              : save === "saving"
                ? "Saving to keyboard…"
                : save === "saved"
                  ? "Saved to keyboard."
                  : keyboard.connection === "dongle"
                    ? "Connected wirelessly."
                    : "Connected by cable."}
          </p>
        </div>
      </div>
    </section>
  );
}
