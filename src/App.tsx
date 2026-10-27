import { useEffect, useState } from "react";
import { detectKeyboard, keyboardInfo, type KeyboardInfo, type KeyboardSummary } from "./lib/keyboard";
import KeysStudio from "./pages/KeysStudio";
import LightingStudio from "./pages/LightingStudio";
import MacrosStudio from "./pages/MacrosStudio";
import ScreenStudio from "./pages/ScreenStudio";

type Page = "screen" | "lighting" | "keys" | "macros";

const NAV: { id: Page; label: string; ready: boolean; icon: string }[] = [
  { id: "screen", label: "Screen", ready: true, icon: "M3 5h18v12H3zM8 21h8M12 17v4" },
  { id: "lighting", label: "Lighting", ready: true, icon: "M12 3v2M5.6 5.6l1.4 1.4M3 12h2M12 8a4 4 0 0 1 2 7.5V18h-4v-2.5A4 4 0 0 1 12 8zM10 21h4" },
  { id: "keys", label: "Keys", ready: true, icon: "M3 7h18v10H3zM7 11h.01M11 11h.01M15 11h.01M8 14h8" },
  { id: "macros", label: "Macros", ready: true, icon: "M5 4l14 8-14 8z" },
];

function Icon({ d }: { d: string }) {
  return (
    <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
      <path d={d} />
    </svg>
  );
}

export default function App() {
  const [page, setPage] = useState<Page>("screen");
  const [keyboard, setKeyboard] = useState<KeyboardSummary | null>(null);
  const [info, setInfo] = useState<KeyboardInfo | null>(null);

  useEffect(() => {
    let alive = true;
    let lastKey = "";
    const poll = async () => {
      const kb = await detectKeyboard().catch(() => null);
      if (!alive) return;
      const key = kb ? `${kb.connection}:${kb.name}:${kb.supported}` : "";
      if (key === lastKey) return;
      lastKey = key;
      setKeyboard(kb);
      setInfo(null);
      if (kb?.supported) {
        keyboardInfo()
          .then((i) => alive && setInfo(i))
          .catch(() => {});
      }
    };
    poll();
    const timer = setInterval(poll, 2000);
    return () => {
      alive = false;
      clearInterval(timer);
    };
  }, []);

  const current = NAV.find((n) => n.id === page)!;

  let detail = "Plug in the keyboard";
  if (keyboard && !keyboard.supported) detail = "Not an F99 MAX";
  else if (keyboard?.connection === "wired") detail = `USB cable${info ? ` · firmware ${info.firmware}` : ""}`;
  else if (keyboard) detail = `Wireless${info ? ` · battery ${info.battery}%` : ""}`;

  return (
    <div className="shell">
      <aside className="sidebar">
        <div className="brand">
          <span className="brand-mark" />
          <div>
            <strong>CustomChrome</strong>
            <small>for AULA F99 MAX</small>
          </div>
        </div>

        <nav>
          {NAV.map((n) => (
            <button key={n.id} className={`nav-item ${page === n.id ? "active" : ""}`} onClick={() => setPage(n.id)}>
              <Icon d={n.icon} />
              <span>{n.label}</span>
              {!n.ready && <em>soon</em>}
            </button>
          ))}
        </nav>

        <div className={`device-card ${!keyboard ? "offline" : !keyboard.supported ? "unsupported" : keyboard.connection}`}>
          <span className="dot" />
          <div>
            <strong>{keyboard ? (keyboard.supported ? "AULA F99 MAX" : keyboard.name) : "No keyboard"}</strong>
            <small>{detail}</small>
          </div>
        </div>
      </aside>

      <main className="content">
        {page === "screen" ? (
          <ScreenStudio keyboard={keyboard} info={info} />
        ) : page === "lighting" ? (
          <LightingStudio keyboard={keyboard} />
        ) : page === "keys" ? (
          <KeysStudio keyboard={keyboard} />
        ) : page === "macros" ? (
          <MacrosStudio keyboard={keyboard} />
        ) : (
          <section className="placeholder">
            <Icon d={current.icon} />
            <h1>{current.label}</h1>
            <p>This page is coming in a later version. The official app's {current.label.toLowerCase()} protocol is already decoded.</p>
          </section>
        )}
      </main>
    </div>
  );
}
