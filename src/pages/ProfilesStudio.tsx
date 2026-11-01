import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import { errorText, loadMacroNames, storeMacroNames, type KeyboardSummary } from "../lib/keyboard";
import { loadPrefs } from "../lib/prefs";

interface ProfileInfo {
  id: string;
  name: string;
}

interface AppLink {
  exe: string;
  label: string;
  profileId: string;
  path: string;
  icon: string;
}

interface ProfileState {
  auto: boolean;
  defaultProfile: string | null;
  profiles: ProfileInfo[];
  apps: AppLink[];
  appliedId: string | null;
  appliedExe: string | null;
  armedId: string | null;
  hasMacros: boolean;
  macroNames: string[];
  restartSuggested: boolean;
}

interface OpenApp {
  exe: string;
  title: string;
  path: string;
  icon: string;
}

function AppMark({ icon, label }: { icon?: string; label: string }) {
  if (icon) return <img className="app-icon" src={icon} alt="" />;
  const letter = label.trim().charAt(0).toUpperCase() || "?";
  return <span className="app-icon letter">{letter}</span>;
}

interface Activity {
  phase: "applying" | "applied" | "captured" | "failed";
  profileId: string;
  profileName: string;
  exe: string;
  macroNames?: string[];
  hasMacros?: boolean;
  restart?: boolean;
}

function rememberMacros(activity: Activity) {
  if (!activity.hasMacros) return "";
  storeMacroNames(activity.macroNames ?? []);
  window.dispatchEvent(new CustomEvent("macro-names"));
  return activity.restart && loadPrefs().macroReminder ? " Restart the keyboard so the macros play." : "";
}

const call = <T,>(cmd: string, args?: Record<string, unknown>) => invoke<T>(cmd, args);

const useNow = (id: string) =>
  async () => {
    const next = await call<ProfileState>("apply_profile", { id });
    if (next.hasMacros) {
      storeMacroNames(next.macroNames);
      window.dispatchEvent(new CustomEvent("macro-names"));
    }
    return next;
  };

export default function ProfilesStudio({ keyboard }: { keyboard: KeyboardSummary | null }) {
  const ready = !!keyboard?.supported;
  const [state, setState] = useState<ProfileState | null>(null);
  const [name, setName] = useState("");
  const [rename, setRename] = useState<{ id: string; name: string } | null>(null);
  const [adding, setAdding] = useState(false);
  const [open, setOpen] = useState<OpenApp[]>([]);
  const [confirmDelete, setConfirmDelete] = useState<string | null>(null);
  const [notice, setNotice] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);

  const refresh = () => call<ProfileState>("profile_state").then(setState).catch((err) => setError(errorText(err)));

  useEffect(() => {
    refresh();
  }, []);

  useEffect(() => {
    const unlisten = listen<Activity>("profile-activity", (event) => {
      const activity = event.payload;
      if (activity.phase === "applying") setNotice(`Putting ${activity.profileName || activity.exe} on the keyboard…`);
      else if (activity.phase === "applied") {
        const extra = rememberMacros(activity);
        if (activity.exe) setNotice(`${activity.profileName} is on, because ${activity.exe} is in front.${extra}`);
      } else if (activity.phase === "failed") setError(`Couldn't switch for ${activity.exe || activity.profileName}.`);
      else if (activity.phase === "captured") {
        setNotice(`${activity.profileName || activity.exe} will use what's on the keyboard now.`);
        setAdding(false);
      }
      refresh();
    });
    return () => {
      unlisten.then((stop) => stop()).catch(() => {});
    };
  }, []);

  useEffect(() => {
    if (!state?.armedId) return;
    const timer = window.setInterval(refresh, 1000);
    return () => clearInterval(timer);
  }, [state?.armedId]);

  const run = async (work: () => Promise<ProfileState>, done: string | ((next: ProfileState) => string)) => {
    setBusy(true);
    setError("");
    try {
      const next = await work();
      setState(next);
      setNotice(typeof done === "function" ? done(next) : done);
    } catch (err) {
      setError(errorText(err));
    } finally {
      setBusy(false);
    }
  };

  const look = async () => {
    setError("");
    try {
      setOpen(await call<OpenApp[]>("open_apps"));
    } catch (err) {
      setError(errorText(err));
    }
  };

  const saveProgram = (app: { exe: string; label: string; path: string }, done: string) =>
    run(async () => {
      const next = await call<ProfileState>("save_program", { exe: app.exe, label: app.label, path: app.path, macroNames: loadMacroNames() });
      setAdding(false);
      return next;
    }, done);

  const manuals = state?.profiles.filter((profile) => !state.apps.some((app) => app.profileId === profile.id)) ?? [];
  let connect = "";
  if (!keyboard) connect = "Connect the keyboard to save a program. You can still remove one.";
  else if (!keyboard.supported) connect = `"${keyboard.name}" isn't an AULA F99 MAX, so profiles stay locked.`;

  return (
    <section className="page">
      <header className="page-header">
        <div>
          <h1>Profiles</h1>
          <p>Each program keeps its own lights, keys and macros. A manual profile is only for when you turn it on yourself.</p>
        </div>
      </header>

      {connect && <div className={`banner ${keyboard?.supported === false ? "warn" : ""}`}>{connect}</div>}
      {error && <div className="banner warn">{error}</div>}
      {notice && !error && <div className="banner">{notice}</div>}
      {state?.armedId && (
        <div className="banner held">
          Click the program you want, then come back. It will save what's on the keyboard now as that program's setup.
          <button className="ghost" disabled={busy} onClick={() => run(() => call("cancel_profile_capture"), "Cancelled.")}>
            Cancel
          </button>
        </div>
      )}

      <div className="profile-detail">
        <div className="card">
          <div className="editor-top">
            <div>
              <div className="card-title">Programs</div>
              <p className="muted">The icon is the program's own. Switching puts that program's setup on the keyboard when it is in front.</p>
            </div>
            <div className="profile-actions">
              <button
                className="ghost"
                disabled={busy || !ready}
                onClick={() => {
                  setAdding((openNow) => !openNow);
                  look();
                }}
              >
                {adding ? "Hide programs" : "Add a program"}
              </button>
              <button className="primary" disabled={busy || !ready} onClick={() => run(() => call("arm_program_capture", { macroNames: loadMacroNames() }), "Switch to the program now.")}>
                I'll switch to it
              </button>
            </div>
          </div>
          {!state?.apps.length && <p className="muted">No program yet. Add one, and the keyboard's current lights, keys and macros become its setup.</p>}
          <div className="app-grid">
            {state?.apps.map((app) => {
              const live = state.appliedId === app.profileId;
              return (
                <article key={app.exe} className="app-card">
                  <div className="app-head">
                    <AppMark icon={app.icon} label={app.label} />
                    <span>
                      <strong>{app.label}</strong>
                      <small>{live ? "On the keyboard now" : app.exe}</small>
                    </span>
                  </div>
                  <div className="app-actions">
                    <button
                      className="ghost"
                      disabled={busy || !ready}
                      onClick={() =>
                        run(useNow(app.profileId), (next) => `Using ${app.label}.${next.restartSuggested && loadPrefs().macroReminder ? " Restart the keyboard so the macros play." : ""}`)
                      }
                    >
                      Use now
                    </button>
                    <button className="ghost" disabled={busy || !ready} onClick={() => saveProgram(app, `${app.label} updated from the keyboard.`)}>
                      Update
                    </button>
                    <button className="ghost danger" disabled={busy} onClick={() => run(() => call("unassign_profile_app", { exe: app.exe }), `Removed ${app.label}.`)}>
                      Remove
                    </button>
                  </div>
                </article>
              );
            })}
          </div>
          {adding && (
            <div className="profile-pick">
              <div className="setting-actions">
                <b>Open programs</b>
                <button className="ghost" disabled={busy} onClick={look}>
                  Look again
                </button>
              </div>
              {!open.length && <p className="muted">No other program with a window is open. Open it, then look again.</p>}
              <div className="app-grid">
                {open.map((app) => (
                  <article key={app.exe} className="app-card">
                    <div className="app-head">
                      <AppMark icon={app.icon} label={app.title} />
                      <span>
                        <strong>{app.title}</strong>
                        <small>{app.exe}</small>
                      </span>
                    </div>
                    <div className="app-actions">
                      <button className="ghost" disabled={busy || !ready} onClick={() => saveProgram({ exe: app.exe, label: app.title, path: app.path }, `${app.title} saved.`)}>
                        Use this
                      </button>
                    </div>
                  </article>
                ))}
              </div>
            </div>
          )}
        </div>

        <details className="card manual-profiles">
          <summary>Manual profiles</summary>
          <p className="muted">These are not tied to a program. Use one when you want that setup yourself.</p>
          <div className="profile-save">
            <input
              value={name}
              placeholder="Name, such as Everyday"
              maxLength={40}
              disabled={busy || !ready}
              onChange={(e) => setName(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter" && name.trim() && ready) {
                  const trimmed = name.trim();
                  run(async () => {
                    const next = await call<ProfileState>("save_profile", { name: trimmed, macroNames: loadMacroNames() });
                    setName("");
                    return next;
                  }, `Saved ${trimmed}.`);
                }
              }}
            />
            <button
              className="ghost"
              disabled={busy || !ready || !name.trim()}
              onClick={() => {
                const trimmed = name.trim();
                run(async () => {
                  const next = await call<ProfileState>("save_profile", { name: trimmed, macroNames: loadMacroNames() });
                  setName("");
                  return next;
                }, `Saved ${trimmed}.`);
              }}
            >
              Save current
            </button>
          </div>
          {!manuals.length && <p className="muted">None yet.</p>}
          <ul className="profile-list">
            {manuals.map((profile) => (
              <li key={profile.id}>
                {rename?.id === profile.id ? (
                  <input
                    value={rename.name}
                    maxLength={40}
                    autoFocus
                    onChange={(e) => setRename({ id: profile.id, name: e.target.value })}
                    onKeyDown={(e) => {
                      if (e.key === "Enter" && rename.name.trim()) {
                        run(async () => {
                          const next = await call<ProfileState>("rename_profile", { id: profile.id, name: rename.name.trim() });
                          setRename(null);
                          return next;
                        }, "Renamed.");
                      }
                    }}
                  />
                ) : (
                  <strong>{profile.name}</strong>
                )}
                <span className="profile-actions">
                  <button
                    className="ghost"
                    disabled={busy || !ready}
                    onClick={() =>
                      run(
                        useNow(profile.id),
                        (next) => `Using ${profile.name}.${next.restartSuggested && loadPrefs().macroReminder ? " Restart the keyboard so the macros play." : ""}`,
                      )
                    }
                  >
                    Use now
                  </button>
                  <button className="ghost" disabled={busy} onClick={() => setRename({ id: profile.id, name: profile.name })}>
                    Rename
                  </button>
                  <button
                    className="ghost danger"
                    disabled={busy}
                    onClick={() => {
                      if (confirmDelete !== profile.id) {
                        setConfirmDelete(profile.id);
                        return;
                      }
                      setConfirmDelete(null);
                      run(() => call("delete_profile", { id: profile.id }), `Deleted ${profile.name}.`);
                    }}
                  >
                    {confirmDelete === profile.id ? "Delete it?" : "Delete"}
                  </button>
                </span>
              </li>
            ))}
          </ul>
        </details>

        <div className="card">
          <div className="card-title">Switching</div>
          <div className="setting-row">
            <div>
              <b>Switch by itself</b>
              <small>When a program in the list above is in front, its own setup goes on the keyboard. This window never triggers a switch. If the keys differ, typing pauses for a moment while they are written.</small>
            </div>
            <button
              className={`switch ${state?.auto ? "on" : ""}`}
              disabled={busy || !state}
              onClick={() => state && run(() => call("set_profile_auto", { on: !state.auto }), state.auto ? "Automatic switching is off." : "Automatic switching is on.")}
            >
              <span />
              {state?.auto ? "On" : "Off"}
            </button>
          </div>
          <div className="setting-row">
            <div>
              <b>Every other program</b>
              <small>A manual profile, used when the program in front does not have its own setup.</small>
            </div>
            <select
              className="profile-select"
              value={manuals.some((profile) => profile.id === state?.defaultProfile) ? state?.defaultProfile ?? "" : ""}
              disabled={busy || !state}
              onChange={(e) => run(() => call("set_default_profile", { id: e.target.value || null }), "Updated.")}
            >
              <option value="">Leave the keyboard as it is</option>
              {manuals.map((profile) => (
                <option key={profile.id} value={profile.id}>
                  {profile.name}
                </option>
              ))}
            </select>
          </div>
        </div>
      </div>
    </section>
  );
}
