# CustomChrome

The user is not a programmer. When they report a problem, read the app's own diagnostics before asking them anything:

- Log: `D:\Apps\CustomChrome\logs\customchrome.log` (older entries in `customchrome.log.old`). Times are UTC; the user is UTC+8.
- Problem reports written from the app's "Report a problem" button: `D:\Apps\CustomChrome\logs\reports\report-*.txt`. Each holds the user's description, the app's view of the keyboard (health check, remapped keys) and the recent log.

`WARN` lines about missed replies or read-back mismatches are early signs of trouble even when the action eventually succeeded.

Writing to the keyboard can break typing: it erases a table when a write starts. Only change the keyboard through `src-tauri/src/aula/device.rs` (which acknowledges every chunk and verifies by read-back), never with ad-hoc scripts, and never flash firmware.
