import { useEffect, useMemo, useRef, useState } from "react";
import { decodeImage, drawFrame, renderAll, scaledDelays, SCREEN_H, SCREEN_W, type Fit, type SourceFrame } from "../lib/gif";
import { errorText, uploadScreen, type KeyboardInfo, type KeyboardSummary } from "../lib/keyboard";

interface Props {
  keyboard: KeyboardSummary | null;
  info: KeyboardInfo | null;
}

type Status = { kind: "idle" } | { kind: "busy"; percent: number } | { kind: "done"; text: string } | { kind: "error"; text: string };

const FITS: { id: Fit; label: string }[] = [
  { id: "cover", label: "Fill" },
  { id: "contain", label: "Fit" },
  { id: "stretch", label: "Stretch" },
];

export default function ScreenStudio({ keyboard, info }: Props) {
  const [fileName, setFileName] = useState("");
  const [frames, setFrames] = useState<SourceFrame[]>([]);
  const [fit, setFit] = useState<Fit>("cover");
  const [background, setBackground] = useState("#000000");
  const [speed, setSpeed] = useState(1);
  const [status, setStatus] = useState<Status>({ kind: "idle" });
  const [dragging, setDragging] = useState(false);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  const frameLimit = Math.min(info?.maxFrames || 255, 255);
  const delays = useMemo(() => scaledDelays(frames, speed), [frames, speed]);
  const opts = useMemo(() => ({ fit, background, speed }), [fit, background, speed]);

  useEffect(() => {
    const ctx = canvasRef.current?.getContext("2d");
    if (!ctx || frames.length === 0) return;
    let index = 0;
    let timer = 0;
    const tick = () => {
      drawFrame(ctx, frames[index].bitmap, opts);
      timer = window.setTimeout(tick, delays[index] * 10);
      index = (index + 1) % frames.length;
    };
    tick();
    return () => clearTimeout(timer);
  }, [frames, delays, opts]);

  useEffect(() => () => frames.forEach((f) => f.bitmap.close()), [frames]);

  async function load(file: File | undefined) {
    if (!file) return;
    if (!file.type.startsWith("image/")) {
      setStatus({ kind: "error", text: "Choose a GIF, PNG, JPG or WebP image." });
      return;
    }
    try {
      setStatus({ kind: "idle" });
      const decoded = await decodeImage(file, frameLimit);
      setFrames(decoded);
      setFileName(file.name);
    } catch (err) {
      setStatus({ kind: "error", text: `Could not read that image: ${errorText(err)}` });
    }
  }

  async function upload() {
    setStatus({ kind: "busy", percent: 0 });
    try {
      const rgba = renderAll(frames, opts);
      const sent = await uploadScreen(rgba, delays, (percent) => setStatus({ kind: "busy", percent }));
      setStatus({ kind: "done", text: `Sent ${sent} frame${sent === 1 ? "" : "s"} to the keyboard.` });
    } catch (err) {
      setStatus({ kind: "error", text: errorText(err) });
    }
  }

  const canUpload = frames.length > 0 && keyboard?.screenUpload && status.kind !== "busy";
  const totalMs = delays.reduce((a, b) => a + b * 10, 0);

  return (
    <section className="page">
      <header className="page-header">
        <div>
          <h1>Screen</h1>
          <p>Put your own GIF or picture on the keyboard's 240×135 display.</p>
        </div>
      </header>

      {keyboard?.connection === "dongle" && (
        <div className="banner warn">
          The keyboard is connected through the wireless dongle. Plug in the USB cable and switch the keyboard to wired mode to change the screen.
        </div>
      )}
      {!keyboard && <div className="banner">Connect the keyboard with its USB cable to upload.</div>}

      <div className="studio">
        <div
          className={`drop ${dragging ? "dragging" : ""} ${frames.length ? "has-image" : ""}`}
          onDragOver={(e) => {
            e.preventDefault();
            setDragging(true);
          }}
          onDragLeave={() => setDragging(false)}
          onDrop={(e) => {
            e.preventDefault();
            setDragging(false);
            load(e.dataTransfer.files[0]);
          }}
          onClick={() => inputRef.current?.click()}
        >
          <div className="bezel">
            <canvas ref={canvasRef} width={SCREEN_W} height={SCREEN_H} />
            {frames.length === 0 && (
              <div className="drop-hint">
                <strong>Drop a GIF here</strong>
                <span>or click to choose a file</span>
              </div>
            )}
          </div>
          <input
            ref={inputRef}
            type="file"
            accept="image/gif,image/png,image/jpeg,image/webp"
            hidden
            onChange={(e) => {
              load(e.target.files?.[0]);
              e.target.value = "";
            }}
          />
        </div>

        <div className="panel">
          <div className="field">
            <label>Image</label>
            <div className="file-meta">
              {frames.length ? (
                <>
                  <strong title={fileName}>{fileName}</strong>
                  <span>
                    {frames.length} frame{frames.length === 1 ? "" : "s"} · {(totalMs / 1000).toFixed(1)} s loop
                    {frames.length >= frameLimit && ` · trimmed to ${frameLimit}`}
                  </span>
                </>
              ) : (
                <span>No image chosen</span>
              )}
            </div>
          </div>

          <div className="field">
            <label>Sizing</label>
            <div className="segmented">
              {FITS.map((f) => (
                <button key={f.id} className={fit === f.id ? "on" : ""} onClick={() => setFit(f.id)}>
                  {f.label}
                </button>
              ))}
            </div>
          </div>

          <div className="field">
            <label>Background</label>
            <div className="color-row">
              <input type="color" value={background} onChange={(e) => setBackground(e.target.value)} />
              <span>{background.toUpperCase()}</span>
            </div>
          </div>

          <div className="field">
            <label>
              Speed <b>{speed.toFixed(2)}×</b>
            </label>
            <input type="range" min={0.25} max={3} step={0.05} value={speed} onChange={(e) => setSpeed(Number(e.target.value))} />
          </div>

          <button className="primary" disabled={!canUpload} onClick={upload}>
            {status.kind === "busy" ? `Uploading ${status.percent}%` : "Send to keyboard"}
          </button>

          {status.kind === "busy" && (
            <div className="progress">
              <span style={{ width: `${status.percent}%` }} />
            </div>
          )}
          {status.kind === "done" && <p className="status ok">{status.text}</p>}
          {status.kind === "error" && <p className="status err">{status.text}</p>}
          <p className="hint">Keep the cable plugged in until the upload finishes.</p>
        </div>
      </div>
    </section>
  );
}
