import { useCallback, useEffect, useRef, useState, type CSSProperties } from "react";
import ColorWheel from "../components/ColorWheel";
import KeyboardPreview, { MiniPreview } from "../components/KeyboardPreview";
import { EFFECT_OFF, EFFECTS, fromHex, LEVEL_MAX, SWATCHES, toHex } from "../lib/effects";
import { errorText, explainKeyboardWait, readLighting, readWhenFree, writeLighting, type KeyboardSummary, type Lighting } from "../lib/keyboard";

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

function Levels({ value, onChange, label }: { value: number; onChange: (v: number) => void; label: string }) {
  return (
    <div className="levels" role="radiogroup" aria-label={label}>
      {Array.from({ length: LEVEL_MAX }, (_, i) => i + 1).map((v) => (
        <button
          key={v}
          role="radio"
          aria-checked={value === v}
          className={v <= value ? "lit" : ""}
          style={{ height: `${10 + v * 4}px` }}
          onClick={() => onChange(v)}
        />
      ))}
    </div>
  );
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
    readWhenFree(readLighting)
      .then((read) => {
        if (!alive) return;
        const l = { ...read, ...rememberedColor() };
        if (l.effect !== EFFECT_OFF) lastEffect.current = l.effect;
        latest.current = l;
        setLighting(l);
      })
      .catch((err) => alive && setLoadError(explainKeyboardWait(err)));
    return () => {
      alive = false;
    };
  }, [keyboardKey]);

  useEffect(() => () => clearTimeout(timer.current), []);

  useEffect(() => {
    if (!lighting) return;
    const color = lighting.rainbow ? "#8b7cff" : toHex(lighting.color);
    document.documentElement.style.setProperty("--live", color);
    window.dispatchEvent(new CustomEvent("key-color", { detail: { color: toHex(lighting.color), rainbow: lighting.rainbow } }));
  }, [lighting]);

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
    const base = latest.current;
    if (!base) return;
    const next = { ...base, ...patch };
    if (next.effect !== EFFECT_OFF) lastEffect.current = next.effect;
    latest.current = next;
    setLighting(next);
    clearTimeout(timer.current);
    timer.current = window.setTimeout(flush, WRITE_DELAY_MS);
  };

  const on = !!lighting && lighting.effect !== EFFECT_OFF;

  const header = (
    <header className="page-header">
      <div>
        <h1>Lighting</h1>
        <p>Changes are saved to the keyboard as you make them.</p>
      </div>
      {lighting && (
        <button
          className={`switch big ${on ? "on" : ""}`}
          role="switch"
          aria-checked={on}
          onClick={() => update({ effect: on ? EFFECT_OFF : lastEffect.current })}
        >
          <span />
          {on ? "Lights on" : "Lights off"}
        </button>
      )}
    </header>
  );

  let notice: string | null = null;
  if (!keyboard) notice = "Connect the keyboard with its USB cable or wireless dongle.";
  else if (!keyboard.supported) notice = `"${keyboard.name}" isn't an AULA F99 MAX, so CustomChrome won't change its lighting.`;
  else if (!lighting) notice = loadError || "Reading the keyboard's lighting…";

  if (notice || !lighting) {
    return (
      <section className="page">
        {header}
        <div className={`banner ${loadError || keyboard?.supported === false ? "warn" : ""}`}>{notice}</div>
      </section>
    );
  }

  const effect = EFFECTS.find((e) => e.id === (on ? lighting.effect : lastEffect.current));
  const hex = toHex(lighting.color);
  const saveText =
    typeof save === "object"
      ? save.error
      : save === "saving"
        ? "Saving…"
        : save === "saved"
          ? "Saved to keyboard"
          : keyboard!.connection === "dongle"
            ? "Wireless"
            : "USB cable";

  return (
    <section className="page">
      {header}

      <div className="hero card">
        <KeyboardPreview lighting={lighting} effect={effect} on={on} />
        <div className="hero-foot">
          <span className="hero-effect">{on ? (effect?.name ?? "Custom") : "Off"}</span>
          <span className={`save-pill ${typeof save === "object" ? "err" : save}`}>
            <i />
            {saveText}
          </span>
        </div>
      </div>

      <div className="lighting">
        <div className="card">
          <div className="card-title">Effects</div>
          <div className={`effect-grid ${on ? "" : "dimmed"}`}>
            {EFFECTS.map((e) => (
              <button
                key={e.id}
                className={`effect ${effect?.id === e.id ? "on" : ""}`}
                onClick={() => update({ effect: e.id, direction: e.direction ? lighting.direction : 0 })}
              >
                <MiniPreview effect={e} lighting={lighting} />
                <span>{e.name}</span>
              </button>
            ))}
          </div>
        </div>

        <div className="side">
          {effect?.usesColor && (
            <div className="card">
              <div className="card-title">
                Colour
                <button
                  className={`switch ${lighting.rainbow ? "on" : ""}`}
                  role="switch"
                  aria-checked={lighting.rainbow}
                  onClick={() => update({ rainbow: !lighting.rainbow })}
                >
                  <span />
                  Rainbow
                </button>
              </div>
              <div className={`wheel-wrap ${lighting.rainbow ? "dimmed" : ""}`}>
                <ColorWheel size={210} value={lighting.color} onChange={(color) => update({ color, rainbow: false })} />
              </div>
              <div className="orbs">
                {SWATCHES.map((s) => (
                  <button
                    key={s}
                    className={`orb ${!lighting.rainbow && hex === s ? "on" : ""}`}
                    style={{ "--orb": s } as CSSProperties}
                    aria-label={s}
                    onClick={() => update({ color: fromHex(s), rainbow: false })}
                  />
                ))}
              </div>
              <div className="hex-row">
                <span className="color-chip" style={{ background: hex }} />
                <input
                  key={hex}
                  className="hex-input"
                  defaultValue={hex}
                  maxLength={7}
                  spellCheck={false}
                  aria-label="Hex colour"
                  onChange={(e) => {
                    const v = e.target.value.trim();
                    if (/^#?[0-9a-fA-F]{6}$/.test(v)) update({ color: fromHex(v.startsWith("#") ? v : `#${v}`), rainbow: false });
                  }}
                />
              </div>
            </div>
          )}

          <div className="card">
            <div className="tune">
              <span>Brightness</span>
              <Levels label="Brightness" value={lighting.brightness} onChange={(brightness) => update({ brightness })} />
            </div>
            {effect?.hasSpeed && (
              <div className="tune">
                <span>Speed</span>
                <Levels label="Speed" value={lighting.speed} onChange={(speed) => update({ speed })} />
              </div>
            )}
            {effect?.direction && (
              <div className="tune">
                <span>Direction</span>
                <div className="pills">
                  <button className={lighting.direction === 1 ? "on" : ""} onClick={() => update({ direction: 1 })}>
                    {effect.direction === "horizontal" ? "← Left" : "↑ Up"}
                  </button>
                  <button className={lighting.direction === 0 ? "on" : ""} onClick={() => update({ direction: 0 })}>
                    {effect.direction === "horizontal" ? "Right →" : "Down ↓"}
                  </button>
                </div>
              </div>
            )}
          </div>

          <p className="strip-tip">
            The light strip by Esc is set on the keyboard: <kbd>Fn</kbd> + <kbd>Right Shift</kbd> changes its effect, <kbd>Fn</kbd> + <kbd>/</kbd> its colour.
          </p>
        </div>
      </div>
    </section>
  );
}
