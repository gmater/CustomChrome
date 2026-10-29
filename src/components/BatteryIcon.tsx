import type { AppPrefs } from "../lib/prefs";

interface Props {
  level: number;
  charging?: boolean;
  percent: AppPrefs["batteryStyle"];
}

/** Status-bar battery: the fill tracks the level, and turns red at 20% or below. */
export default function BatteryIcon({ level, charging = false, percent }: Props) {
  const pct = Math.max(0, Math.min(100, Math.round(level)));
  return (
    <span className={`battery ${pct <= 20 ? "low" : ""} ${charging ? "charging" : ""} ${percent}`} title={`${pct}%${charging ? ", charging" : ""}`}>
      {percent === "beside" && <span className="battery-pct">{pct}%</span>}
      <span className="battery-shell">
        <span className="battery-body">
          <span className="battery-fill" style={{ width: `${pct}%` }} />
          {percent === "inside" && <span className="battery-in">{pct}</span>}
          {charging && (
            <svg className="battery-bolt" viewBox="0 0 24 24" aria-hidden="true">
              <path d="M13 2 4 14h7l-1 8 10-14h-7l0-6z" />
            </svg>
          )}
        </span>
        <span className="battery-nub" />
      </span>
    </span>
  );
}
