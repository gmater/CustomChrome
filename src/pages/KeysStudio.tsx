import { useEffect, useMemo, useState, type CSSProperties } from "react";
import { describe, GROUPS, macroEntry, macroName, MACRO_MODES, sameEntry, type Assignable, type KeyEntry } from "../lib/keycodes";
import { assignKey, errorText, factoryKeys, loadMacroNames, readMacros, resetKeys, type KeyboardSummary } from "../lib/keyboard";
import { FN_INDEX, KEYBOARD_UNITS_H, KEYBOARD_UNITS_W, KEYS, SCREEN, STRIP } from "../lib/layout";

interface Props {
  keyboard: KeyboardSummary | null;
}

type Status = "idle" | "saving" | "saved" | { error: string };

const pct = (v: number, of: number) => `${(v / of) * 100}%`;

export default function KeysStudio({ keyboard }: Props) {
  const [entries, setEntries] = useState<KeyEntry[] | null>(null);
  const [factory, setFactory] = useState<Map<number, KeyEntry>>(new Map());
  const [loadError, setLoadError] = useState("");
  const [selected, setSelected] = useState<number | null>(null);
  const [group, setGroup] = useState(GROUPS[0].name);
  const [query, setQuery] = useState("");
  const [status, setStatus] = useState<Status>("idle");
  const [confirmReset, setConfirmReset] = useState(false);
  const [macroCount, setMacroCount] = useState(0);
  const [macroMode, setMacroMode] = useState(0);
  const [repeats, setRepeats] = useState(3);
  const macroNames = useMemo(loadMacroNames, [keyboard]);

  const keyboardKey = keyboard?.supported ? `${keyboard.connection}:${keyboard.name}` : "";

  useEffect(() => {
    setEntries(null);
    setLoadError("");
    setStatus("idle");
    if (!keyboardKey) return;
    let alive = true;
    Promise.all([readMacros(), factoryKeys()])
      .then(([state, defaults]) => {
        if (!alive) return;
        setFactory(new Map(defaults));
        setMacroCount(state.macros.length);
        setEntries(state.keys);
      })
      .catch((err) => alive && setLoadError(errorText(err)));
    return () => {
      alive = false;
    };
  }, [keyboardKey]);

  const run = async (task: () => Promise<KeyEntry[]>) => {
    setStatus("saving");
    try {
      setEntries(await task());
      setStatus("saved");
    } catch (err) {
      setStatus({ error: errorText(err) });
    }
  };

  const isRemapped = (index: number) => {
    const def = factory.get(index);
    return !!entries && !!def && !sameEntry(entries[index], def);
  };

  const remappedCount = useMemo(() => KEYS.filter((k) => k.index !== FN_INDEX && isRemapped(k.index)).length, [entries, factory]);

  const macroItems: Assignable[] = useMemo(
    () => Array.from({ length: macroCount }, (_, i) => ({ label: macroName(macroNames, i), entry: macroEntry(i, macroMode, repeats) })),
    [macroCount, macroNames, macroMode, repeats],
  );
  const groups = useMemo(() => [...GROUPS, { name: "Macros", items: macroItems }], [macroItems]);
  const showingMacros = !query.trim() && group === "Macros";

  const options = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return groups.find((g) => g.name === group)!.items;
    return groups.flatMap((g) => g.items).filter((a) => a.label.toLowerCase().includes(q));
  }, [group, query, groups]);

  const header = (
    <header className="page-header">
      <div>
        <h1>Keys</h1>
        <p>Click a key, then choose what it should do. Changes are saved to the keyboard straight away.</p>
      </div>
      {entries && (
        <button
          className={`ghost ${confirmReset ? "danger" : ""}`}
          disabled={status === "saving" || (remappedCount === 0 && !confirmReset)}
          onBlur={() => setConfirmReset(false)}
          onClick={() => {
            if (!confirmReset) return setConfirmReset(true);
            setConfirmReset(false);
            run(resetKeys);
          }}
        >
          {confirmReset ? `Reset ${remappedCount} key${remappedCount === 1 ? "" : "s"}?` : "Reset all keys"}
        </button>
      )}
    </header>
  );

  let notice: string | null = null;
  if (!keyboard) notice = "Connect the keyboard with its USB cable or wireless dongle.";
  else if (!keyboard.supported) notice = `"${keyboard.name}" isn't an AULA F99 MAX, so CustomChrome won't change its keys.`;
  else if (!entries) notice = loadError ? `Couldn't read the keyboard's keys: ${loadError}` : "Reading the keyboard's keys…";

  if (notice || !entries) {
    return (
      <section className="page">
        {header}
        <div className={`banner ${loadError || keyboard?.supported === false ? "warn" : ""}`}>{notice}</div>
      </section>
    );
  }

  const selectedKey = KEYS.find((k) => k.index === selected);
  const current = selected != null ? entries[selected] : null;
  const statusText =
    typeof status === "object" ? status.error : status === "saving" ? "Saving…" : status === "saved" ? "Saved to keyboard" : `${remappedCount} remapped`;

  return (
    <section className="page">
      {header}

      <div className="hero card">
        <div className="kb keymap">
          <div className="kb-plate">
            {KEYS.map((k) => {
              const locked = k.index === FN_INDEX;
              const remapped = !locked && isRemapped(k.index);
              return (
                <button
                  key={k.index}
                  className={`key ${selected === k.index ? "selected" : ""} ${remapped ? "remapped" : ""} ${locked ? "locked" : ""}`}
                  disabled={locked}
                  title={locked ? "Fn can't be remapped, so its shortcuts always keep working" : undefined}
                  style={
                    {
                      left: pct(k.x, KEYBOARD_UNITS_W),
                      top: pct(k.y, KEYBOARD_UNITS_H),
                      width: pct(k.w, KEYBOARD_UNITS_W),
                      height: pct(1, KEYBOARD_UNITS_H),
                    } as CSSProperties
                  }
                  onClick={() => setSelected(k.index)}
                >
                  <span className="cap">
                    <span className="legend">{k.label}</span>
                    {remapped && <span className="remap-label">{describe(entries[k.index], macroNames)}</span>}
                  </span>
                </button>
              );
            })}
            <div
              className="kb-strip"
              style={{ left: pct(STRIP.x, KEYBOARD_UNITS_W), top: pct(STRIP.y, KEYBOARD_UNITS_H), width: pct(STRIP.w, KEYBOARD_UNITS_W), height: pct(STRIP.h, KEYBOARD_UNITS_H) }}
            />
            <div
              className="kb-screen"
              style={{ left: pct(SCREEN.x, KEYBOARD_UNITS_W), top: pct(SCREEN.y, KEYBOARD_UNITS_H), width: pct(SCREEN.w, KEYBOARD_UNITS_W), height: pct(SCREEN.h, KEYBOARD_UNITS_H) }}
            >
              <span>F99 MAX</span>
            </div>
          </div>
        </div>
        <div className="hero-foot">
          <span className="hero-effect">{selectedKey ? `${selectedKey.label || "Space"} → ${describe(current!, macroNames)}` : "Select a key"}</span>
          <span className={`save-pill ${typeof status === "object" ? "err" : status}`}>
            <i />
            {statusText}
          </span>
        </div>
      </div>

      <div className="lighting">
        <div className={`card picker ${selected == null ? "dimmed" : ""}`}>
          <div className="picker-top">
            <input
              className="search"
              placeholder="Search keys, media, mouse…"
              value={query}
              spellCheck={false}
              onChange={(e) => setQuery(e.target.value)}
            />
          </div>
          {!query && (
            <div className="chips">
              {groups.map((g) => (
                <button key={g.name} className={group === g.name ? "on" : ""} onClick={() => setGroup(g.name)}>
                  {g.name}
                </button>
              ))}
            </div>
          )}
          {showingMacros && macroCount > 0 && (
            <div className="macro-mode">
              <div className="pills">
                {MACRO_MODES.map((m) => (
                  <button key={m.mode} className={macroMode === m.mode ? "on" : ""} onClick={() => setMacroMode(m.mode)}>
                    {m.label}
                  </button>
                ))}
              </div>
              {macroMode === 1 && (
                <label className="delay-field">
                  <input
                    type="number"
                    min={1}
                    max={255}
                    value={repeats}
                    onChange={(e) => setRepeats(Math.max(1, Math.min(255, Math.round(Number(e.target.value)) || 1)))}
                  />
                  times
                </label>
              )}
            </div>
          )}
          {showingMacros && macroCount === 0 && <p className="empty">No macros on the keyboard yet. Record one on the Macros page.</p>}
          <div className="option-grid">
            {options.map((a) => (
              <button
                key={a.entry.join(",")}
                className={`option ${current && sameEntry(current, a.entry) ? "on" : ""}`}
                disabled={selected == null || status === "saving"}
                onClick={() => run(() => assignKey(selected!, a.entry))}
              >
                {a.label}
              </button>
            ))}
            {options.length === 0 && query && <p className="empty">Nothing matches “{query}”.</p>}
          </div>
        </div>

        <div className="side">
          <div className="card key-detail">
            <div className="card-title">Selected key</div>
            {selectedKey ? (
              <>
                <div className="key-big">{selectedKey.label || "Space"}</div>
                <div className="key-row">
                  <span>Does</span>
                  <b>{describe(current!, macroNames)}</b>
                </div>
                <div className="key-row">
                  <span>Factory</span>
                  <b>{describe(factory.get(selectedKey.index)!, macroNames)}</b>
                </div>
                <button
                  className="ghost wide"
                  disabled={!isRemapped(selectedKey.index) || status === "saving"}
                  onClick={() => run(() => assignKey(selectedKey.index, null))}
                >
                  Restore default
                </button>
              </>
            ) : (
              <p className="muted">Click any key on the keyboard above. Fn is locked so its shortcuts always work.</p>
            )}
          </div>
          <p className="strip-tip">
            Remaps are stored on the keyboard itself, not in this app.
          </p>
        </div>
      </div>
    </section>
  );
}
