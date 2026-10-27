import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import { errorText, logEvent } from "./lib/keyboard";
import "./styles.css";

if (import.meta.env.DEV && !("__TAURI_INTERNALS__" in window && "invoke" in (window as any).__TAURI_INTERNALS__)) {
  const { installMockKeyboard } = await import("./dev/mockKeyboard");
  installMockKeyboard();
}

window.addEventListener("error", (e) => logEvent("error", `${e.message} at ${e.filename}:${e.lineno}`));
window.addEventListener("unhandledrejection", (e) => logEvent("error", `unhandled: ${errorText(e.reason)}`));

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
