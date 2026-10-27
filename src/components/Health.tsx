import { useState } from "react";
import { hidLabel } from "../lib/keycodes";
import { errorText, repairKeyboard, saveReport, type Health, type Issue } from "../lib/keyboard";
import { KEYS } from "../lib/layout";

const keyName = (index: number) => {
  const k = KEYS.find((k) => k.index === index);
  return k ? k.label || "Space" : `hidden slot ${index}`;
};

export function describeIssues(issues: Issue[]): string[] {
  const lines: string[] = [];
  const damaged = issues.filter((i) => i.kind === "damagedKey");
  if (damaged.length > 3) lines.push(`${damaged.length} keys hold damaged settings and may not type.`);
  else for (const i of damaged) lines.push(`${keyName(i.key)} holds a damaged setting and may not type.`);
  for (const i of issues) {
    if (i.kind === "missingMacro") lines.push(`${keyName(i.key)} plays macro M${i.slot + 1}, which no longer exists.`);
    if (i.kind === "macroLeavesHeld")
      lines.push(`Macro M${i.slot + 1} leaves ${i.codes.map((c) => hidLabel(c, false)).join(", ")} held down after it plays.`);
    if (i.kind === "badMacroStep") lines.push(`Macro M${i.slot + 1} has a step the keyboard can't play.`);
  }
  return lines;
}

interface BannerProps {
  health: Health;
  onFixed: (h: Health) => void;
}

export function HealthBanner({ health, onFixed }: BannerProps) {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  if (!health.issues.length) return null;
  return (
    <div className="health-banner">
      <div>
        <strong>CustomChrome found a problem on your keyboard</strong>
        <ul>
          {describeIssues(health.issues).map((l) => (
            <li key={l}>{l}</li>
          ))}
        </ul>
        {error && <p className="health-error">The fix didn't work: {error}</p>}
      </div>
      <button
        className="primary"
        disabled={busy}
        onClick={async () => {
          setBusy(true);
          setError("");
          try {
            onFixed(await repairKeyboard());
          } catch (err) {
            setError(errorText(err));
          } finally {
            setBusy(false);
          }
        }}
      >
        {busy ? "Fixing…" : "Fix it"}
      </button>
    </div>
  );
}

interface ReportProps {
  context: () => string;
  onClose: () => void;
}

export function ReportDialog({ context, onClose }: ReportProps) {
  const [text, setText] = useState("");
  const [saved, setSaved] = useState("");
  const [error, setError] = useState("");
  return (
    <div className="dialog-backdrop" onClick={onClose}>
      <div className="card dialog" onClick={(e) => e.stopPropagation()}>
        {saved ? (
          <>
            <h2>Report saved</h2>
            <p className="muted">
              It's on this PC only, together with CustomChrome's log. In Cursor, tell the assistant
              <b> "check my CustomChrome report"</b> and it will read it.
            </p>
            <code className="report-path">{saved}</code>
            <button className="primary" onClick={onClose}>
              Done
            </button>
          </>
        ) : (
          <>
            <h2>Report a problem</h2>
            <p className="muted">Describe what went wrong in your own words: what you did, and what the keyboard or app did.</p>
            <textarea
              autoFocus
              value={text}
              placeholder="e.g. I pressed numpad / and it kept typing A until I unplugged the keyboard"
              onChange={(e) => setText(e.target.value)}
            />
            {error && <p className="health-error">{error}</p>}
            <div className="dialog-actions">
              <button className="ghost" onClick={onClose}>
                Cancel
              </button>
              <button
                className="primary"
                disabled={!text.trim()}
                onClick={() =>
                  saveReport(text, context())
                    .then(setSaved)
                    .catch((err) => setError(errorText(err)))
                }
              >
                Save report
              </button>
            </div>
          </>
        )}
      </div>
    </div>
  );
}
