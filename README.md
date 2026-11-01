# CustomChrome beta

CustomChrome is an offline app for the **AULA F99 MAX**. It changes the screen, the lights, the keys, and the macros, and it keeps those settings on the keyboard.

There is no account, no telemetry, and no updater. The app does not download or flash firmware. Copy `CustomChrome.exe` to a Windows 10 or Windows 11 PC and open it. Rust, Node, and Tauri are not needed to run it.

Version **0.1.1** is the beta. The keyboard it is written for is the F99 MAX on its current firmware (tested on 1.22).

## Requirements

- Windows 10 or Windows 11, 64-bit.
- Microsoft Edge WebView2, which is already part of current Windows 10 and 11. If a PC is missing it, install the Evergreen WebView2 Runtime from Microsoft. Nothing else has to be installed.
- An AULA F99 MAX, connected by its USB cable or its wireless dongle.
- About 3.2 MB of disk space for the portable exe.

The USB cable is required to change the screen. Lighting, keys, macros, the clock, and wireless sleep also work over the dongle. A different keyboard is detected and left alone.

What stays where:

| Stored on the keyboard | Stored in this Windows account |
| --- | --- |
| Screen picture | Last page you had open |
| Lighting | Accent colour |
| Key remaps | Battery icon style |
| Macros | Macro names |
| Screen clock and wireless sleep | The reminder after saving a macro |

Program setups and manual profiles are saved in `profiles.json` next to the exe.

Unplugging the keyboard does not clear remaps, macros, lighting, or the screen picture. Deleting the exe does not clear them either. App choices above are lost if that Windows account's app data for CustomChrome is cleared.

## Screen

Puts a still image or a GIF on the keyboard's **240×135** display.

- Accepts GIF, PNG, JPG, and WebP. Drag a file in or browse for one.
- Preview matches the screen before anything is sent.
- Sizing is Fill, Fit, or Stretch. Fit and Fill can use a background colour around the picture.
- Playback speed runs from 0.25× to 3×.
- The keyboard reports how many frames it can store. This generation holds **104**. Longer GIFs are trimmed to that limit. Each frame is held between 10 ms and 510 ms.
- Upload shows a progress bar. Leave the USB cable plugged in until it finishes.
- The dongle cannot update the screen. Plug in the cable and switch the keyboard to wired mode.

## Lighting

Nineteen effects, plus lights off. The choice, colour, brightness, speed, and direction are written to the keyboard. Brightness and speed each have six levels. A colour wheel, ten swatches, and a hex field set the colour. Rainbow is available when the effect uses your colour.

The large keyboard on the page is a preview of the effect that is on. Pressing a key on the PC plays the reactive effects in that preview. The small tile for each effect stays still until you hover it, or until that effect is the one turned on.

The side light strip is drawn on the preview. The keyboard itself changes that strip with **Fn + Right Shift**. CustomChrome does not program the strip, and it does not offer per-key custom colours.

| Effect | What it does |
| --- | --- |
| Static | Every key holds the colour you picked. No speed control. |
| Single key on | The board stays dark. The key you press lights, then fades. |
| Single key off | The board stays lit. The key you press goes dark, then fades back. |
| Starry sky | Every key stays on. One random key blinks once, then the blink moves to another key. Speed sets how fast it moves. |
| Snowfall | Specks fall down the keys. |
| Blossom | Each key has its own colour, and the colours slowly change. Your chosen colour is not used. |
| Breathing | The whole board fades in and out. |
| Spectrum cycle | Every key shares one hue, and that hue cycles. Your chosen colour is not used. |
| Colour fountain | Light surges upward from the bottom. |
| Colour crossing | Bands travel up and down and cross each other. Direction is up or down. |
| Wave | A band washes across the board. Direction is left or right. |
| Turning peaks | Light travels on a diagonal path and brightens more than once. Direction is left or right. |
| One touch fire | The row of the key you press lights from that key out to both ends. Speed sets the delay. |
| Two birds | Light shoots both ways along the row of the key you press. |
| Ripples | A ring spreads out from the key you press. |
| Endless flow | A steady stream moves across the board. Direction is left or right. |
| Layered mountains | Several bright bands are stacked and move together. |
| Gentle rain | Drops fall down the keys. Direction slants the fall left or right. |
| Back and forth | A wash travels across, then comes back. |

## Keys

The page shows the real F99 MAX layout, including the knob, the screen, and the light strip.

- Click a key to see what it currently does. The category list opens on that assignment.
- Assign letters, numbers, function keys, editing keys, arrows, symbols, modifiers (Ctrl, Shift, Alt, Win), numpad keys, media keys, or mouse buttons.
- Media keys include playback, volume, mute, and shortcuts such as Calculator, Mail, and the browser buttons.
- Mouse actions are left, right, and middle click, back, forward, and scroll up or down.
- Assign any saved macro, and choose how it plays.
- Remapped keys are tinted and labelled on this page, so they are easy to find.
- **Remapped keys** opens a list of each changed key and what it does. Choosing a row selects that key.
- **Fn cannot be remapped**, so Fn shortcuts keep working.
- One key can be restored, or every key can be restored to the factory layout.
- The knob's volume actions are part of that factory layout.

## Macros

Macros are recorded, edited, and stored on the keyboard. Names are remembered by the app on this PC.

- Record keystrokes and mouse buttons. Keep the real timing, or give every step the same delay.
- Edit the list: change a delay, remove a step, or insert a key or mouse button.
- A macro that would leave a key or button held down cannot be saved until those releases are added. The editor can add the missing releases.
- The memory bar shows how full the keyboard's macro area is. The cap is 156 words, about 75 key presses. A macro that does not fit is refused.
- Delete a macro. Keys that played it return to the factory key, and later macros stay pointed at the right recording.
- From the Keys page, a macro can play once, repeat a set number of times, or keep going until the key is pressed again.
- After a save or delete, the app suggests restarting the keyboard so it plays the new recording. That reminder can be turned off in Settings.

## Profiles

A program and a manual profile are different.

**Programs** each keep their own lights, keys, and macros. The list shows the program's own Windows icon.

- **Add a program** saves what the keyboard is doing right now as that program's setup. **I'll switch to it** does the same for the next program you click.
- **Use now** puts that setup on the keyboard. **Update** replaces it with the keyboard's current lights, keys, and macros. **Remove** deletes that program's setup.
- **Switch by itself** applies a program's setup when that program is the window in front. This window does not trigger a switch. If the keys differ, typing can pause for a moment while they are written.
- **Every other program** can use a manual profile, or leave the keyboard as it is.

**Manual profiles** are tucked under the program list. They are not tied to a program. Save the current keyboard under a name, then turn that setup on yourself with **Use now**.

The screen picture is not part of a profile.

## Settings

**Keyboard**

- Connection (USB cable or wireless dongle), model, and firmware version.
- Battery icon in the sidebar. The fill follows the level the keyboard reports, and a bolt shows when it is charging. The percentage can be hidden, shown beside the icon, or drawn inside it, the same three layouts as a phone status bar.

**Screen clock**

- 12-hour or 24-hour time on the keyboard screen.
- **Use this PC's time** copies the computer's date and time onto the screen.

**Wireless sleep**

- Shown only while the dongle is connected.
- Off leaves the lights on. On turns them off after 1 to 30 minutes without typing.

**This app**

- Reopen on the last page you used.
- Accent colour for buttons and remapped keys, or the same colour as the keys. Rainbow lighting has no single key colour, so the accent stays violet.
- The reminder after saving a macro.
- **Open log folder** and **Report a problem**. Both stay on this PC. A report includes your note, the keyboard's health check, and the recent log. Nothing is uploaded.

**Factory reset**

- Type `RESET`, then confirm. This wipes lighting, key remaps, macros, and the screen picture on the keyboard. CustomChrome cannot undo it. It does not install firmware.

## If something looks wrong

When a supported keyboard connects, CustomChrome reads it and checks for damage:

- Keys whose settings are unreadable and may not type.
- A key set to play a macro that is no longer stored.
- A macro that leaves a key held down.
- A macro step the keyboard cannot play.

A banner explains what it found and offers **Fix it**. Writes to the keyboard are acknowledged and read back, so a missed reply is retried instead of leaving a half-written key table.

Logs are saved beside the exe as `logs\customchrome.log`. Reports are saved in `logs\reports\`.

## 0.1.1

- Program setups and manual profiles, with each program's own icon.
- Keyboard replies are still accepted when Windows adds an extra byte in front of them.
- Pages wait their turn instead of reporting that the keyboard is busy while another read is still going.

## Not in this beta

- Firmware downloads or flashing.
- Per-key custom lighting.
- Control of the side light strip from the app.
- Screen upload over the wireless dongle.
- Swapping the Windows and Mac modifier layout.
- An installer, an account, or an update check.

## Building from source

Only needed if you are changing the app. Running the portable exe does not require this.

- Node.js and Rust, plus the Tauri prerequisites for Windows.
- `npm install`
- `npm run tauri build -- --no-bundle`

The exe is written to `src-tauri\target\release\customchrome.exe`.
