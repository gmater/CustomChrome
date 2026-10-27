import { useEffect, useRef, useState } from "react";
import { hidFromCode, hidLabel, macroName, MACRO_KEY_GROUPS, MACRO_MOUSE } from "../lib/keycodes";
import {
  deleteMacro,
  errorText,
  loadMacroNames,
  macroCapacity,
  readMacros,
  saveMacro,
  storeMacroNames,
  type KeyboardSummary,
  type MacroEvent,
  type MacroState,
} from "../lib/keyboard";

interface Props {
  keyboard: KeyboardSummary | null;
}

type Status = "idle" | "saving" | "saved" | { error: string };

const DELAY_MAX = 65535;
const clampDelay = (v: number) => Math.max(0, Math.min(DELAY_MAX, Math.round(v) || 0));

/** Keys and buttons a macro leaves pressed, which would otherwise stay held after it plays. */
function stillHeld(events: MacroEvent[]): MacroEvent[] {
  const held = new Map<string, MacroEvent>();
  for (const e of events) {
    const id = `${e.mouse}:${e.code}`;
    if (e.down) held.set(id, e);
    else held.delete(id);
  }
  return [...held.values()];
}

const withReleases = (events: MacroEvent[], delay: number) => [
  ...events,
  ...stillHeld(events).map((e) => ({ ...e, down: false, delay })),
];

export default function MacrosStudio({ keyboard }: Props) {
  const [state, setState] = useState<MacroState | null>(null);
  const [capacity, setCapacity] = useState(156);
  const [names, setNames] = useState<string[]>(loadMacroNames);
  const [loadError, setLoadError] = useState("");
  const [selected, setSelected] = useState<number | null>(null);
  const [draft, setDraft] = useState<MacroEvent[]>([]);
  const [draftName, setDraftName] = useState("");
  const [dirty, setDirty] = useState(false);
  const [recording, setRecording] = useState(false);
  const [keepTiming, setKeepTiming] = useState(true);
  const [fixedDelay, setFixedDelay] = useState(20);
  const [addKey, setAddKey] = useState("k:4");
  const [status, setStatus] = useState<Status>("idle");
  const [confirmDelete, setConfirmDelete] = useState(false);
  const lastAt = useRef(0);

  const keyboardKey = keyboard?.supported ? `${keyboard.connection}:${keyboard.name}` : "";

  useEffect(() => {
    setState(null);
    setLoadError("");
    setStatus("idle");
    if (!keyboardKey) return;
    let alive = true;
    Promise.all([readMacros(), macroCapacity()])
      .then(([s, cap]) => {
        if (!alive) return;
        setCapacity(cap);
        setState(s);
      })
      .catch((err) => alive && setLoadError(errorText(err)));
    return () => {
      alive = false;
    };
  }, [keyboardKey]);

  const macros = state?.macros ?? [];

  const open = (index: number) => {
    setSelected(index);
    setDraft(index < macros.length ? macros[index].map((e) => ({ ...e })) : []);
    setDraftName(macroName(names, index));
    setDirty(index >= macros.length);
    setRecording(false);
    setConfirmDelete(false);
    setStatus("idle");
  };

  const edit = (events: MacroEvent[]) => {
    setDraft(events);
    setDirty(true);
  };

  useEffect(() => {
    if (!recording) return;
    lastAt.current = 0;
    const onKey = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      if (e.repeat) return;
      const code = hidFromCode(e.code);
      if (code == null) return;
      const now = performance.now();
      const gap = lastAt.current ? now - lastAt.current : 0;
      lastAt.current = now;
      setDraft((d) => {
        const next = d.slice();
        if (next.length) next[next.length - 1] = { ...next[next.length - 1], delay: keepTiming ? clampDelay(gap) : fixedDelay };
        next.push({ code, down: e.type === "keydown", mouse: false, delay: fixedDelay });
        return next;
      });
      setDirty(true);
    };
    window.addEventListener("keydown", onKey, true);
    window.addEventListener("keyup", onKey, true);
    return () => {
      window.removeEventListener("keydown", onKey, true);
      window.removeEventListener("keyup", onKey, true);
    };
  }, [recording, keepTiming, fixedDelay]);

  const otherWords = macros.reduce((sum, m, i) => (i === selected ? sum : sum + 1 + m.length), 0);
  const usedWords = otherWords + (selected != null ? 1 + draft.length : 0);
  const overCapacity = usedWords > capacity;
  const held = stillHeld(draft);

  const persistName = (index: number, name: string) => {
    const next = names.slice();
    next[index] = name.trim();
    setNames(next);
    storeMacroNames(next);
  };

  const save = async () => {
    if (selected == null || !draft.length || overCapacity) return;
    setRecording(false);
    setStatus("saving");
    try {
      const s = await saveMacro(selected, draft);
      persistName(selected, draftName);
      setState(s);
      setDirty(false);
      setStatus("saved");
    } catch (err) {
      setStatus({ error: errorText(err) });
    }
  };

  const remove = async () => {
    if (selected == null) return;
    if (selected >= macros.length) {
      setSelected(null);
      setDirty(false);
      return;
    }
    setStatus("saving");
    try {
      const s = await deleteMacro(selected);
      const next = names.slice();
      next.splice(selected, 1);
      setNames(next);
      storeMacroNames(next);
      setState(s);
      setSelected(null);
      setDirty(false);
      setConfirmDelete(false);
      setStatus("saved");
    } catch (err) {
      setStatus({ error: errorText(err) });
    }
  };

  const addTap = () => {
    const [kind, raw] = addKey.split(":");
    const code = Number(raw);
    const mouse = kind === "m";
    edit([...draft, { code, down: true, mouse, delay: fixedDelay }, { code, down: false, mouse, delay: fixedDelay }]);
  };

  const boundKeys = (index: number) => state?.keys.filter((e) => e[0] === 6 && e[1] === index).length ?? 0;

  const header = (
    <header className="page-header">
      <div>
        <h1>Macros</h1>
        <p>Record a sequence of key presses, then put it on any key from the Keys page.</p>
      </div>
      {state && (
        <button
          className="primary"
          disabled={dirty || status === "saving" || macros.length >= 100 || otherWords + 1 >= capacity}
          onClick={() => open(macros.length)}
        >
          New macro
        </button>
      )}
    </header>
  );

  let notice: string | null = null;
  if (!keyboard) notice = "Connect the keyboard with its USB cable or wireless dongle.";
  else if (!keyboard.supported) notice = `"${keyboard.name}" isn't an AULA F99 MAX, so CustomChrome won't change its macros.`;
  else if (!state) notice = loadError ? `Couldn't read the keyboard's macros: ${loadError}` : "Reading the keyboard's macros…";

  if (notice || !state) {
    return (
      <section className="page">
        {header}
        <div className={`banner ${loadError || keyboard?.supported === false ? "warn" : ""}`}>{notice}</div>
      </section>
    );
  }

  const listed = selected != null && selected >= macros.length ? [...macros, draft] : macros;
  const statusText =
    typeof status === "object" ? status.error : status === "saving" ? "Saving…" : status === "saved" ? "Saved to keyboard" : dirty ? "Unsaved changes" : "Up to date";

  return (
    <section className="page">
      {header}

      <div className="macros">
        <div className="card macro-list">
          <div className="card-title">On the keyboard</div>
          {listed.length === 0 && <p className="muted">No macros yet. Click New macro to record one.</p>}
          {listed.map((m, i) => {
            const bound = boundKeys(i);
            return (
              <button
                key={i}
                className={`macro-item ${selected === i ? "on" : ""}`}
                disabled={dirty && selected !== i}
                onClick={() => selected !== i && open(i)}
              >
                <span className="macro-badge">M{i + 1}</span>
                <span className="macro-meta">
                  <b>{selected === i ? draftName || macroName(names, i) : macroName(names, i)}</b>
                  <small>
                    {i >= macros.length ? "Not saved yet" : `${m.filter((e) => e.down).length} key presses`}
                    {bound > 0 && ` · on ${bound} key${bound === 1 ? "" : "s"}`}
                  </small>
                </span>
              </button>
            );
          })}
          <div className="memory">
            <div className="memory-bar">
              <i style={{ width: `${Math.min(100, (usedWords / capacity) * 100)}%` }} className={overCapacity ? "over" : ""} />
            </div>
            <span>
              Memory {Math.round((usedWords / capacity) * 100)}% used · about {Math.max(0, Math.floor((capacity - usedWords) / 2))} key presses left
            </span>
          </div>
        </div>

        {selected == null ? (
          <div className="card macro-empty">
            <p className="muted">Pick a macro on the left, or create a new one.</p>
            <p className="strip-tip">To use a macro, open Keys, click a key and choose it under Macros.</p>
          </div>
        ) : (
          <div className="card macro-editor">
            <div className="editor-top">
              <input
                className="search name-input"
                value={draftName}
                maxLength={32}
                placeholder={`Macro ${selected + 1}`}
                onChange={(e) => {
                  setDraftName(e.target.value);
                  setDirty(true);
                }}
              />
              <span className={`save-pill ${typeof status === "object" ? "err" : status}`}>
                <i />
                {statusText}
              </span>
            </div>

            <div className="record-bar">
              <button
                className={`record ${recording ? "on" : ""}`}
                onClick={() => {
                  if (recording) setDraft((d) => withReleases(d, fixedDelay));
                  setRecording(!recording);
                }}
              >
                <i />
                {recording ? "Stop recording" : "Record"}
              </button>
              <label className="check">
                <input type="checkbox" checked={keepTiming} onChange={(e) => setKeepTiming(e.target.checked)} />
                Keep my timing
              </label>
              <label className="delay-field">
                {keepTiming ? "Last step" : "Every step"}
                <input
                  type="number"
                  min={0}
                  max={DELAY_MAX}
                  value={fixedDelay}
                  onChange={(e) => setFixedDelay(clampDelay(Number(e.target.value)))}
                />
                ms
              </label>
              <button className="ghost" disabled={!draft.length || recording} onClick={() => edit([])}>
                Clear
              </button>
            </div>
            {recording && <div className="banner rec">Recording: type on your keyboard. Press Stop recording when you're done.</div>}

            <ol className="steps">
              {draft.map((e, i) => (
                <li key={i} className={e.down ? "down" : "up"}>
                  <span className="step-kind">{e.down ? "Press" : "Release"}</span>
                  <span className="step-key">{hidLabel(e.code, e.mouse)}</span>
                  <label className="step-delay">
                    then wait
                    <input
                      type="number"
                      min={0}
                      max={DELAY_MAX}
                      value={e.delay}
                      disabled={recording}
                      onChange={(ev) => edit(draft.map((d, j) => (j === i ? { ...d, delay: clampDelay(Number(ev.target.value)) } : d)))}
                    />
                    ms
                  </label>
                  <button className="step-del" title="Remove step" disabled={recording} onClick={() => edit(draft.filter((_, j) => j !== i))}>
                    ×
                  </button>
                </li>
              ))}
              {draft.length === 0 && <li className="steps-empty">No steps yet. Press Record, or add a key below.</li>}
            </ol>

            <div className="add-step">
              <select value={addKey} disabled={recording} onChange={(e) => setAddKey(e.target.value)}>
                {MACRO_KEY_GROUPS.map((g) => (
                  <optgroup key={g.name} label={g.name}>
                    {g.keys.map((k) => (
                      <option key={k.code} value={`k:${k.code}`}>
                        {k.label}
                      </option>
                    ))}
                  </optgroup>
                ))}
                <optgroup label="Mouse">
                  {MACRO_MOUSE.map((m) => (
                    <option key={m.code} value={`m:${m.code}`}>
                      {m.label}
                    </option>
                  ))}
                </optgroup>
              </select>
              <button className="ghost" disabled={recording} onClick={addTap}>
                Add press and release
              </button>
            </div>

            {!recording && held.length > 0 && (
              <div className="banner warn held">
                <span>
                  {held.map((e) => hidLabel(e.code, e.mouse)).join(", ")} {held.length === 1 ? "is" : "are"} pressed but never
                  released, so {held.length === 1 ? "it" : "they"} would stay held down after the macro plays.
                </span>
                <button className="ghost" onClick={() => edit(withReleases(draft, fixedDelay))}>
                  Add releases
                </button>
              </div>
            )}
            {overCapacity && <div className="banner warn">This macro doesn't fit in the keyboard's memory. Remove some steps.</div>}

            <div className="editor-foot">
              <button
                className={`ghost ${confirmDelete ? "danger" : ""}`}
                disabled={status === "saving"}
                onBlur={() => setConfirmDelete(false)}
                onClick={() => (confirmDelete || selected >= macros.length ? remove() : setConfirmDelete(true))}
              >
                {selected >= macros.length ? "Discard" : confirmDelete ? (boundKeys(selected) ? "Delete and free its keys?" : "Delete macro?") : "Delete"}
              </button>
              <span className="grow" />
              {dirty && selected < macros.length && (
                <button className="ghost" disabled={status === "saving"} onClick={() => open(selected)}>
                  Undo changes
                </button>
              )}
              <button className="primary" disabled={!dirty || !draft.length || overCapacity || recording || held.length > 0 || status === "saving"} onClick={save}>
                Save to keyboard
              </button>
            </div>
          </div>
        )}
      </div>
    </section>
  );
}
