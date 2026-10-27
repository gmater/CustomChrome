import { mockIPC } from "@tauri-apps/api/mocks";

/** Lets `npm run dev` render in a plain browser for UI work; never bundled into the app. */
export function installMockKeyboard() {
  document.body.style.background = "linear-gradient(135deg, #1d2030, #121318 60%, #1a1424)";
  let lighting = { effect: 0x0b, color: [255, 45, 85], rainbow: true, brightness: 5, speed: 3, direction: 0 };
  mockIPC((cmd, args) => {
    switch (cmd) {
      case "detect_keyboard":
        return { name: "F99 MAX Dongle", connection: "dongle", supported: true, screenUpload: false };
      case "keyboard_info":
        return { firmware: "1.22", battery: 56, maxFrames: 104 };
      case "read_lighting":
        return lighting;
      case "write_lighting":
        lighting = (args as { lighting: typeof lighting }).lighting;
        return new Promise((resolve) => setTimeout(() => resolve(lighting), 150));
      default:
        return null;
    }
  });
}
