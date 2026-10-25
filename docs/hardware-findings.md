# Hardware Findings — Apple Magic Keyboard with Numeric Keypad (A1843)

> Status: **Verified on physical hardware** (Milestone 0 spike, 2026-10-25).
> Probe tool: `crates/hid-probe` (`hid-probe list`, `hid-probe dump`, `hid-probe battery`).

## 1. Device Identification

| Property | USB Value | Bluetooth Value |
|---|---|---|
| Manufacturer | `Apple Inc.` | `Apple Inc.` |
| Product Name | `Magic Keyboard with Numeric Keypad` | `Magic Keyboard with Numeric Keypad` |
| Vendor ID (`VID`) | **`0x05AC`** (USB-IF Apple Inc.) | **`0x004C`** (Bluetooth SIG Apple Inc., `VID&0001004c`) |
| Product ID (`PID`) | `0x026C` | `0x026C` |
| Revision (`VersionNumber`) | `0x0852` | `0x0000` |
| Tested Layout | ISO (Italian Windows keyboard layout active) | ISO (Italian Windows keyboard layout active) |

> [!IMPORTANT]
> Over Bluetooth Classic (`{00001124-0000-1000-8000-00805f9b34fb}`), the A1843 identifies itself with Apple's **Bluetooth SIG Company ID `0x004C`** (`VID&0001004c_PID&026c`) rather than Apple's USB Vendor ID `0x05AC`. `magicbridge_core::is_apple_vendor` checks both `0x05AC` and `0x004C`.

---

## 2. Top-Level HID Collections (USB)

When connected via Lightning-to-USB cable, the A1843 enumerates as a composite USB device (`USB\VID_05AC&PID_026C`) with two USB interfaces (`MI_00` "Device Management" and `MI_01` "Keyboard / Boot") and **5 top-level HID collections**:

| # | Interface & Collection | Product String | Top-Level Usage | Report Lengths (In / Out / Feat) | User-Mode `ReadFile` | Purpose & Report Fields |
|---|---|---|---|---|---|---|
| `[0]` | `MI_00&Col01` | `"Device Management"` | `0xFF00:0x000B` (Vendor) | `5 / 0 / 0` | `OK` | Input `0x9a` (8-bit × 1; `GetInputReport` returns `9a 01 00 00 00`), Input `0xe0` (8-bit × 4). |
| `[1]` | `MI_00&Col02` | `"Device Management"` | `0xFF00:0x0014` (Vendor) | `3 / 0 / 0` | `OK` | **Battery System** (`id=0x90`, 3 bytes):<br>• `0x0085:0x0044` Charging (1-bit button)<br>• `0x0085:0x0046` Discharging (1-bit button)<br>• `0x0085:0x0061` AC Present (1-bit button)<br>• `0x0085:0x0065` `AbsoluteStateOfCharge` (8-bit value, `0..255`). |
| `[2]` | `MI_01&Col01` (`\kbd`) | `"Keyboard / Boot"` | `0x0001:0x0006` (Generic Desktop / Keyboard) | `10 / 2 / 0` | **Denied (`0x80070005`)** | Main keyboard (`id=0x01`, 10B in / 2B LED out):<br>• `0x0007:0x0000..0x00FF` Keys<br>• `0x0007:0x00E0..0x00E7` Modifiers<br>• `0x000C:0x00B8` **Consumer Eject** (1-bit button)<br>• `0x00FF:0x0003` **Apple Fn** (7-bit value, logical `0..1`). |
| `[3]` | `MI_01&Col02` | `"Keyboard / Boot"` | `0x000C:0x0001` (Consumer Control) | `2 / 0 / 4` | `OK` | Input `0x52` (2B): Fast Forward (`0xB3`), Rewind (`0xB4`), Next (`0xB5`), Prev (`0xB6`), Play/Pause (`0xCD`).<br>Feature `0x09` (4B): `0xFF01:0x000B` (`GetFeature` returns `09 01 00 00`). |
| `[4]` | `MI_01&Col03` | `"Keyboard / Boot"` | `0xFF00:0x0006` (Vendor) | `65 / 0 / 0` | `OK` | Input `0x3f` (8-bit × 64). |

### 2.1 Top-Level HID Collections (Bluetooth Classic)

When connected over Bluetooth Classic (`{00001124-0000-1000-8000-00805f9b34fb}_VID&0001004c_PID&026c`), the A1843 exposes **2 top-level HID collections**:

| # | Collection | Product String | Top-Level Usage | Report Lengths (In / Out / Feat) | User-Mode `ReadFile` | Purpose & Report Fields |
|---|---|---|---|---|---|---|
| `[0]` | `Col01` (`\kbd`) | `"Magic Keyboard with Numeric Keypad"` | `0x0001:0x0006` (Generic Desktop / Keyboard) | `10 / 2 / 65` | **Denied (`0x80070005`)** | Main keyboard (`id=0x01`, 10B in / 2B LED out):<br>• `0x0007:0x0000..0x00FF` Keys<br>• `0x0007:0x00E0..0x00E7` Modifiers<br>• `0x000C:0x00B8` **Consumer Eject** (1-bit button)<br>• `0xFF00:0x0003` **Apple Fn** (1-bit button)<br>• `0xFF01:0x0003` Vendor (1-bit button)<br>Feature `0x55` (65B): `0xFF02:0x0055`. |
| `[1]` | `Col02` | `"Magic Keyboard with Numeric Keypad"` | `0xFF00:0x0014` (Vendor) | `3 / 0 / 0` | `OK` | **Battery System** (`id=0x90`, 3 bytes): identical to USB `MI_00&Col02` (`0x85:0x44`, `0x85:0x46`, `0x85:0x61`, `0x85:0x65`). |

---

## 3. Why Native `Fn` and `Eject` Are Invisible to User-Mode Windows

1. **Descriptor placement:** Apple placed both `Apple Fn` (`0x00FF:0x0003`) and `Consumer Eject` (`0x000C:0x00B8`) inside **Input Report `0x01`** of the main `0x0001:0x0006` (Generic Desktop / Keyboard) collection (`MI_01&Col01`), **not** in the separate Consumer Control (`MI_01&Col02`) or Vendor (`MI_00&Col01`, `MI_01&Col03`) collections.
2. **Exclusive kernel lock by `kbdhid.sys`:** Windows' inbox `kbdhid.sys` driver opens all `0x0001:0x0006` collections exclusively (`CreateFileW` with `GENERIC_READ` fails with `ERROR_ACCESS_DENIED`). `HidD_GetInputReport(0x01)` on a zero-access query handle fails with Win32 error `1` (`ERROR_INVALID_FUNCTION`).
3. **Usage filtering in `kbdhid.sys`:** `kbdhid.sys` only translates Usage Page `0x07` usages into `KEYBOARD_INPUT_DATA` scancodes (which feed both `WH_KEYBOARD_LL` and `WM_INPUT` `RIM_TYPEKEYBOARD`). Non-`0x07` usages (`0x00FF:0x0003` Fn and `0x000C:0x00B8` Eject) in Report `0x01` are silently discarded in kernel mode.
4. **No hardware Fn translation:** Pressing `Fn` alone, `Fn + Arrows`, `Fn + Backspace`, `Fn + Enter`, or `Fn + F1..F12` produces **no vendor/consumer report** on any of the readable collections (`[0]`, `[1]`, `[3]`, `[4]`), and the keyboard sends the unmodified base key (`Left/Right/Up/Down`, `Backspace`, `Enter`, `F1..F12`).
5. **Architecture consequence (resolves 🔬 in `F1.1`, `F1.4`, `F4.1`, `03-architecture.md`):**
   - Intercepting physical `Fn` or `Eject` on Windows requires a kernel filter driver above/below `kbdhid.sys`, which is explicitly forbidden by `NFR-S1` and `03-architecture.md` (admin requirement, BSOD risk, anti-cheat incompatibility).
   - MagicBridge therefore uses the **Fn fallback key (`F1.4`)** as its Fn-layer modifier (default: **Right Option / `VK_RMENU`**, user-configurable to any key such as Right Control or `F19`).

---

## 4. Key Event Ground Truth (`WH_KEYBOARD_LL` & `RAWKB`)

All keys below were verified in `target\usb-dump.log`:

| Physical Key (A1843) | `WH_KEYBOARD_LL` `vkCode` | `scanCode` | `LLKHF_EXTENDED` (`ext`) | Notes |
|---|---|---|---|---|
| `F1` – `F10` | `0x70` – `0x79` (`VK_F1`–`VK_F10`) | `0x3B` – `0x44` | `0` | Sent directly regardless of physical `Fn`. |
| `F11`, `F12` | `0x7A`, `0x7B` (`VK_F11`, `VK_F12`) | `0x57`, `0x58` | `0` | Sent directly regardless of physical `Fn`. |
| `F13` – `F19` | `0x7C` – `0x82` (`VK_F13`–`VK_F19`) | `0x64` – `0x6A` | `0` | Clean, distinct VKs and scancodes (`0x64..0x6A`). |
| `Clear` (above Numpad `7`) | `0x90` (`VK_NUMLOCK`) | `0x45` | `1` (`RAWKB` `e0=0`) | Reported as Num Lock (`"BLOC NUM"`). |
| `Backspace` | `0x08` (`VK_BACK`) | `0x0E` | `0` | Standard Backspace. |
| `Delete ⌦` (Forward Delete) | `0x2E` (`VK_DELETE`) | `0x53` | `1` | Standard Extended Delete (`"CANCELLA"`). |
| `Return` (Main Enter) | `0x0D` (`VK_RETURN`) | `0x1C` | `0` | Numpad Enter uses `vk=0x0D`, `sc=0x1C`, `ext=1`. |
| `←`, `→`, `↑`, `↓` | `0x25`, `0x27`, `0x26`, `0x28` | `0x4B`, `0x4D`, `0x48`, `0x50` | `1` | Extended arrow keys. |
| `Caps Lock` | `0x14` (`VK_CAPITAL`) | `0x3A` | `0` | Normal key down/up events (`~50–65 ms` tap). |
| `Left Shift` | `0xA0` (`VK_LSHIFT`) | `0x2A` | `0` | `RAWKB` reports `vk=0x10`. |
| `Right Shift` | `0xA1` (`VK_RSHIFT`) | `0x36` | `1` (`RAWKB` `e0=0`) | Note `LLKHF_EXTENDED=1` in `WH_KEYBOARD_LL`. |
| `Left Control` | `0xA2` (`VK_LCONTROL`) | `0x1D` | `0` | Standard Left Ctrl. |
| `Right Control` | `0xA3` (`VK_RCONTROL`) | `0x1D` | `1` | Standard Extended Right Ctrl. |
| `Left Option` | `0xA4` (`VK_LMENU`) | `0x38` | `0` | Sets `LLKHF_ALTDOWN` (`WM_SYSKEYDOWN`). |
| `Right Option` (`AltGr` on ISO) | `0xA5` (`VK_RMENU`) | `0x38` | `1` | **Preceded on Italian/ISO layouts by a synthetic `VK_LCONTROL` (`vk=0xA2`, `sc=0x21D`, `ext=0`)!** See §4.1. |
| `Left Command` | `0x5B` (`VK_LWIN`) | `0x5B` | `1` | Physically adjacent to Spacebar. |
| `Right Command` | `0x5C` (`VK_RWIN`) | `0x5C` | `1` | Physically adjacent to Spacebar. |

### 4.1 Critical ISO / `AltGr` Quirk (`sc = 0x21D`)
When an ISO layout with `AltGr` (such as Italian) is active in Windows, pressing physical **Right Option** (`VK_RMENU`, `vk=0xA5`, `sc=0x38`, `ext=1`) causes the Windows keyboard layout driver to synthesize a fake **Left Control** event ~0.8 ms earlier:
```text
HOOK down vk=0xa2 LCtrl sc=0x21d ext=0 alt=1 sys=1
HOOK down vk=0xa5 RAlt  sc=0x38  ext=1 alt=1 sys=1
...
HOOK up   vk=0xa2 LCtrl sc=0x21d ext=0 alt=0 sys=0
HOOK up   vk=0xa5 RAlt  sc=0x38  ext=1 alt=0 sys=0
```
Notice `scanCode == 0x21D` (as opposed to `0x1D` for physical `LCtrl`).
**Requirement for M2/M3:** Whenever `VK_RMENU` (`Right Option`) is remapped (e.g., to `VK_RWIN` in the Windows modifier preset, or used as the `Fn` modifier key), the hook **must** also swallow any `VK_LCONTROL` event with `scanCode == 0x21D` so that `LCtrl` is not spuriously held down.

---

## 5. Battery Reporting (M0.5)

- **Identical HID Report `0x90` on Both USB and Bluetooth:**
  On both USB (`MI_00&Col02`) and Bluetooth (`Col02`), the keyboard exposes top-level collection `0xFF00:0x0014` with Input Report `0x90` (3 bytes). Calling `HidD_GetInputReport` with `buf = [0x90, 0, 0]` on a query or read handle succeeds on **both transports** and returns:
  - `buf[0] = 0x90` (Report ID)
  - `buf[1]` = status bitmask: bit 0 (`0x01`) = Charging (`0x85:0x44`), bit 1 (`0x02`) = Discharging (`0x85:0x46`), bit 2 (`0x04`) = AC Present (`0x85:0x61`)
  - `buf[2]` = battery percentage `0..=100` (`0x85:0x65` `AbsoluteStateOfCharge`).
  - **Observed on USB:** `90 01 64` → `100%`, `status = 0x01` (charging).
  - **Observed on Bluetooth:** `90 00 64` → `100%`, `status = 0x00` (on battery).
- **Why Windows Settings doesn't show battery %:** Windows' `PKEY_BLUETOOTH_BATTERY` property is not populated on the A1843 `BTHENUM` node because Apple wraps the Battery System (`0x85`) fields inside a vendor top-level collection (`0xFF00:0x0014`). `HidD_GetInputReport(0x90)` on `0xFF00:0x0014` is therefore the authoritative battery source on both USB and Bluetooth (with `PKEY_BLUETOOTH_BATTERY` kept as a secondary fallback).

---

## 6. Device Filtering Timing (M0.6)

- Across every keystroke in `target\usb-dump.log`, `WH_KEYBOARD_LL` (`HOOK`) fired **before** `WM_INPUT` (`RAWKB`) by **+0.43 ms to +1.76 ms** (typical ~0.8 ms).
- Because `WH_KEYBOARD_LL` is invoked synchronously before `WM_INPUT` is queued to any window thread, a low-level hook cannot inspect the `WM_INPUT` `hDevice` handle of the current keystroke without blocking the input thread (which would violate the `< 0.1 ms` budget in `NFR-P3` and risk hook removal by Windows).
- **Decision:** Use the v1 architecture default: remapping applies while an Apple keyboard (`VID 0x05AC`) is present on USB or Bluetooth (tracked via event-driven device notifications) and automatically pauses when no Apple keyboard is connected.
