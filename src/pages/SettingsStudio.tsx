import { useEffect, useState } from "react";
import { ReportDialog } from "../components/Health";
import BatteryIcon from "../components/BatteryIcon";
import {
  errorText,
  factoryReset,
  openLogs,
  readBoard,
  syncClock,
  writeBoard,
  type BoardSettings,
  type KeyboardInfo,
  type KeyboardSummary,
} from "../lib/keyboard";
import { ACCENT_CHOICES, accentColor, applyAccent, loadKeyLook, storePrefs, type AppPrefs, type KeyLook } from "../lib/prefs";

interface Props {
  keyboard: KeyboardSummary | null;
  info: KeyboardInfo | null;
  reportContext: () => string;
  prefs: AppPrefs;
  onPrefs: (prefs: AppPrefs) => void;
}

type Status = "idle" | "saving" | "saved" | { error: string };

export default function SettingsStudio({ keyboard, info, reportContext, prefs, onPrefs }: Props) {
  const [board, setBoard] = useState<BoardSettings | null>(null);
  const [loadError, setLoadError] = useState("");
  const [status, setStatus] = useState<Status>("idle");
  const [reporting, setReporting] = useState(false);
  const [keyLook, setKeyLook] = useState<KeyLook>(loadKeyLook);
  const [resetText, setResetText] = useState("");
  const [confirmReset, setConfirmReset] = useState(false);

  const keyboardKey = keyboard?.supported ? `${keyboard.connection}:${keyboard.name}` : "";
  const wireless = keyboard?.connection === "dongle";

  useEffect(() => {
    setBoard(null);
    setLoadError("");
    setStatus("idle");
    if (!keyboardKey) return;
    let alive = true;
    readBoard()
      .then((b) => alive && setBoard(b))
      .catch((err) => alive && setLoadError(errorText(err)));
    return () => {
      alive = false;
    };
  }, [keyboardKey]);

  useEffect(() => {
    const onColor = (event: Event) => setKeyLook((event as CustomEvent<KeyLook>).detail);
    window.addEventListener("key-color", onColor);
    return () => window.removeEventListener("key-color", onColor);
  }, []);

  const saveBoard = async (next: BoardSettings) => {
    setBoard(next);
    setStatus("saving");
    try {
      setBoard(await writeBoard(next));
      setStatus("saved");
    } catch (err) {
      setStatus({ error: errorText(err) });
    }
  };

  const setClock = async () => {
    const now = new Date();
    setStatus("saving");
    try {
      await syncClock({
        year: now.getFullYear(),
        month: now.getMonth() + 1,
        day: now.getDate(),
        hour: now.getHours(),
        minute: now.getMinutes(),
        second: now.getSeconds(),
        weekday: now.getDay(),
      });
      setStatus("saved");
    } catch (err) {
      setStatus({ error: errorText(err) });
    }
  };

  const updatePrefs = (patch: Partial<AppPrefs>) => {
    const next = { ...prefs, ...patch };
    storePrefs(next);
    onPrefs(next);
    if (patch.accent || patch.matchKeys !== undefined) applyAccent(accentColor(next, keyLook));
  };

  let notice: string | null = null;
  if (!keyboard) notice = "Connect the keyboard with its USB cable or wireless dongle.";
  else if (!keyboard.supported) notice = `"${keyboard.name}" isn't an AULA F99 MAX, so these keyboard settings stay locked.`;

  const statusText =
    typeof status === "object" ? status.error : status === "saving" ? "Saving…" : status === "saved" ? "Saved to keyboard" : "";

  const connection = !keyboard ? "Not connected" : keyboard.connection === "wired" ? "USB cable" : "Wireless dongle";

  return (
    <section className="page">
      <header className="page-header">
        <div>
          <h1>Settings</h1>
          <p>Keyboard options from AULA that apply to the F99 MAX, and a few choices for this app.</p>
        </div>
        {statusText && (
          <span className={`save-pill ${typeof status === "object" ? "err" : status}`}>
            <i />
            {statusText}
          </span>
        )}
      </header>

      <div className="settings">
        <div className="card">
          <div className="card-title">Keyboard</div>
          {notice && <div className={`banner ${keyboard?.supported === false ? "warn" : ""}`}>{notice}</div>}
          <div className="setting-row">
            <div>
              <b>Connection</b>
              <small>{keyboard?.supported ? "AULA F99 MAX" : keyboard?.name || "Nothing plugged in"}</small>
            </div>
            <span>{connection}</span>
          </div>
          <div className="setting-row">
            <div>
              <b>Firmware</b>
              <small>Reported by the keyboard</small>
            </div>
            <span>{info?.firmware ?? "—"}</span>
          </div>
          <div className="setting-row">
            <div>
              <b>Battery icon</b>
              <small>Same choices as a phone's status bar. The fill follows the keyboard's battery while this app is open.</small>
            </div>
            <div className="battery-setting">
              <BatteryIcon level={wireless && info ? info.battery : 56} charging={info?.charging ?? false} percent={prefs.batteryStyle} />
              <div className="pills">
                {(
                  [
                    ["hidden", "Icon only"],
                    ["beside", "Next to icon"],
                    ["inside", "Inside icon"],
                  ] as const
                ).map(([id, label]) => (
                  <button key={id} className={prefs.batteryStyle === id ? "on" : ""} onClick={() => updatePrefs({ batteryStyle: id })}>
                    {label}
                  </button>
                ))}
              </div>
            </div>
          </div>
        </div>

        <div className={`card ${!board ? "dimmed" : ""}`}>
          <div className="card-title">Screen clock</div>
          {loadError && <div className="banner warn">{loadError}</div>}
          <div className="setting-row">
            <div>
              <b>Clock format</b>
              <small>How the time is written on the screen</small>
            </div>
            <div className="pills">
              <button className={board?.hour24 ? "on" : ""} disabled={!board || status === "saving"} onClick={() => board && !board.hour24 && saveBoard({ ...board, hour24: true })}>
                24-hour
              </button>
              <button className={board && !board.hour24 ? "on" : ""} disabled={!board || status === "saving"} onClick={() => board && board.hour24 && saveBoard({ ...board, hour24: false })}>
                12-hour
              </button>
            </div>
          </div>
          <div className="setting-row">
            <div>
              <b>Set the clock</b>
              <small>Copies this PC's current date and time onto the screen</small>
            </div>
            <button className="ghost" disabled={!board || status === "saving"} onClick={setClock}>
              Use this PC's time
            </button>
          </div>
          {wireless && (
            <div className="setting-row">
              <div>
                <b>Wireless sleep</b>
                <small>Minutes of no typing before the lights turn off. Off keeps them on.</small>
              </div>
              <div className="sleep-control">
                <button
                  className={`switch ${board && board.sleepMinutes > 0 ? "on" : ""}`}
                  disabled={!board || status === "saving"}
                  onClick={() => board && saveBoard({ ...board, sleepMinutes: board.sleepMinutes > 0 ? 0 : 5 })}
                >
                  <span />
                  {board && board.sleepMinutes > 0 ? "On" : "Off"}
                </button>
                {board && board.sleepMinutes > 0 && (
                  <label className="delay-field">
                    <input
                      type="number"
                      min={1}
                      max={30}
                      value={board.sleepMinutes}
                      disabled={status === "saving"}
                      onChange={(e) => {
                        const n = Math.max(1, Math.min(30, Math.round(Number(e.target.value)) || 1));
                        setBoard({ ...board, sleepMinutes: n });
                      }}
                      onBlur={() => board && saveBoard(board)}
                    />
                    min
                  </label>
                )}
              </div>
            </div>
          )}
          {!wireless && keyboard?.supported && <p className="strip-tip">Wireless sleep shows up when the keyboard is on its dongle.</p>}
        </div>

        <div className="card">
          <div className="card-title">This app</div>
          <div className="setting-row">
            <div>
              <b>Remember the last page</b>
              <small>Reopen CustomChrome on the page you were using</small>
            </div>
            <button className={`switch ${prefs.rememberPage ? "on" : ""}`} onClick={() => updatePrefs({ rememberPage: !prefs.rememberPage })}>
              <span />
              {prefs.rememberPage ? "On" : "Off"}
            </button>
          </div>
          <div className="setting-row">
            <div>
              <b>Accent colour</b>
              <small>Highlight on buttons and remapped keys. Same as the keys follows the colour you set in Lighting.</small>
            </div>
            <div className="accents">
              <button
                className={`keys ${prefs.matchKeys ? "on" : ""} ${keyLook.rainbow ? "rainbow" : ""}`}
                style={keyLook.rainbow ? undefined : { background: keyLook.color }}
                title="Same as the keys"
                onClick={() => updatePrefs({ matchKeys: true })}
              />
              {ACCENT_CHOICES.map((color) => (
                <button
                  key={color}
                  className={!prefs.matchKeys && prefs.accent.toLowerCase() === color ? "on" : ""}
                  style={{ background: color }}
                  title={color}
                  onClick={() => updatePrefs({ accent: color, matchKeys: false })}
                />
              ))}
            </div>
          </div>
          <div className="setting-row">
            <div>
              <b>Remind me after a macro</b>
              <small>After saving a macro, suggest restarting the keyboard so it uses the new one</small>
            </div>
            <button className={`switch ${prefs.macroReminder ? "on" : ""}`} onClick={() => updatePrefs({ macroReminder: !prefs.macroReminder })}>
              <span />
              {prefs.macroReminder ? "On" : "Off"}
            </button>
          </div>
          <div className="setting-row">
            <div>
              <b>Log and reports</b>
              <small>Saved on this PC, next to CustomChrome. Nothing is sent online.</small>
            </div>
            <div className="setting-actions">
              <button className="ghost" onClick={() => openLogs().catch(() => {})}>
                Open log folder
              </button>
              <button className="ghost" onClick={() => setReporting(true)}>
                Report a problem
              </button>
            </div>
          </div>
        </div>

        <div className="card danger-card">
          <div className="card-title">Factory reset</div>
          <p className="muted">
            Puts the keyboard back to factory: lighting, key remaps, macros and the screen picture are wiped. This cannot be undone from CustomChrome.
          </p>
          <label className="delay-field reset-type">
            Type RESET to unlock
            <input value={resetText} spellCheck={false} onChange={(e) => setResetText(e.target.value)} />
          </label>
          <button
            className={`ghost danger ${confirmReset ? "danger" : ""}`}
            disabled={!keyboard?.supported || resetText !== "RESET" || status === "saving"}
            onClick={() => {
              if (!confirmReset) {
                setConfirmReset(true);
                return;
              }
              setConfirmReset(false);
              setStatus("saving");
              factoryReset()
                .then(() => {
                  setStatus("saved");
                  setResetText("");
                  return readBoard().then(setBoard);
                })
                .catch((err) => setStatus({ error: errorText(err) }));
            }}
          >
            {confirmReset ? "Wipe the keyboard now?" : "Reset the keyboard"}
          </button>
        </div>
      </div>

      {reporting && <ReportDialog context={reportContext} onClose={() => setReporting(false)} />}
    </section>
  );
}
