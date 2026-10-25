# MagicBridge User Guide ⌨️ 🌉

**Comprehensive User Manual & Technical Guide for the Apple Magic Keyboard with Numeric Keypad (A1843) on Windows 10 & 11**

---

## Table of Contents
1. [Introduction & Overview](#1-introduction--overview)
2. [Quick Start & Installation](#2-quick-start--installation)
3. [Connecting the Keyboard (USB & Bluetooth)](#3-connecting-the-keyboard-usb--bluetooth)
4. [System Tray Controls](#4-system-tray-controls)
5. [The Fn Layer & Keyboard Combos](#5-the-fn-layer--keyboard-combos)
6. [Top Row & Media Keys](#6-top-row--media-keys)
7. [Modifier Layout Presets](#7-modifier-layout-presets)
8. [Missing Windows Keys & Special Functions](#8-missing-windows-keys--special-functions)
9. [Profiles & Multi-Language Layouts](#9-profiles--multi-language-layouts)
10. [Battery Monitoring & Toast Alerts](#10-battery-monitoring--toast-alerts)
11. [Gaming & Anti-Cheat Compatibility](#11-gaming--anti-cheat-compatibility)
12. [Known Limitations & Workarounds](#12-known-limitations--workarounds)
13. [Uninstallation](#13-uninstallation)

---

## 1. Introduction & Overview

**MagicBridge** is a native, ultra-lightweight Windows 10 and 11 utility tailored specifically for the **Apple Magic Keyboard with Numeric Keypad (Model A1843)**.

When connected to Windows, Apple keyboards normally suffer from several frustrating limitations:
- The physical `Fn` key is locked and ignored by the Windows kernel.
- The `Cmd` and `Option` modifier positions clash with standard Windows PC keyboards (`Ctrl - Win - Alt`).
- Missing critical keys: no `Print Screen`, `Scroll Lock`, `Pause`, or dedicated `Insert`.
- The top-row function and media keys do not toggle naturally.
- Windows does not display battery percentage for Apple Bluetooth keyboards.

MagicBridge fixes all of these issues while maintaining uncompromising system integrity:
- **0% Idle CPU:** Purely event-driven Win32 messaging and hooks. Zero polling loops.
- **Anti-Cheat Safe:** No kernel drivers (`.sys`), no DLL injection, no memory manipulation.
- **Zero Background GUI:** The settings window runs only when open. Closing it leaves zero memory footprint.
- **100% Offline & Private:** Zero network requests, zero telemetry, zero analytics.

---

## 2. Quick Start & Installation

### System Requirements
- **Operating System:** Windows 10 (version 1903 or later) or Windows 11 (64-bit x64).
- **Permissions:** Standard user account (no Administrator privileges required).
- **Hardware:** Apple Magic Keyboard with Numeric Keypad (Model A1843), connected via Lightning-to-USB cable or Bluetooth.

### Method 1: Microsoft WinGet (Recommended)
Install directly using Windows Package Manager:
```powershell
# Install from official repository:
winget install DooperDrew.MagicBridge

# Or install from local repo manifest:
winget install --manifest packaging/winget/manifests/d/DooperDrew/MagicBridge/0.2.0/
```

### Method 2: Scoop Package Manager
Install via Scoop for clean user-space installation and automatic environment handling:
```powershell
scoop install https://raw.githubusercontent.com/DooperDrew/magicbridge/main/packaging/scoop/magicbridge.json
```

### Method 3: Guided Setup Wizard (Zero-Elevation GUI)
MagicBridge provides a zero-elevation graphical setup wizard:
1. Download `MagicBridge-v<version>-windows-x64.zip` from [GitHub Releases](https://github.com/DooperDrew/magicbridge/releases/latest).
2. Extract the archive, and double-click `setup.cmd` (or open PowerShell and run `.\setup-wizard.ps1`).
3. The guided setup wizard allows you to:
   - Select installation destination (default: `%LOCALAPPDATA%\Programs\MagicBridge`, no Administrator privileges needed).
   - Toggle Start Menu and Desktop shortcuts.
   - Configure automatic launch on Windows sign-in (per-user registry `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`).
   - Automatically launch the settings wizard upon completion.
4. MagicBridge registers cleanly in Windows **Installed apps** (Settings &rarr; Apps / Control Panel) with full uninstallation support.

For unattended or headless environments:
```powershell
# Silent zero-touch installation:
.\setup-wizard.ps1 -Silent

# Console-mode interactive setup:
.\setup-wizard.ps1 -NoGui

# Lightweight script installer:
.\install.ps1
```

### Method 4: Portable Run (No Installation Required)
1. Download `MagicBridge-v<version>-windows-x64.zip` from the GitHub Releases page.
2. Extract the archive into any folder of your choice (e.g. `C:\Tools\MagicBridge` or `%LOCALAPPDATA%\MagicBridge`).
3. Double-click `magicbridge-agent.exe`.
4. A keyboard icon will appear in your Windows System Tray (notification area next to the clock).
5. Right-click the tray icon and select **Open Settings** (or launch `magicbridge-settings.exe`) to customize your layout.

---

## 3. Connecting the Keyboard (USB & Bluetooth)

The A1843 keyboard can be used interchangeably over a Lightning-to-USB cable or Bluetooth Classic.

### Connecting via Lightning-to-USB Cable
1. Connect the keyboard to your PC using an Apple Lightning-to-USB cable.
2. Windows detects the keyboard as a standard USB Composite Device (`VID: 0x05AC`, `PID: 0x026C`).
3. MagicBridge automatically detects the keyboard arrival, engages remapping, and begins monitoring battery charging status.
4. The keyboard battery charges continuously while plugged in.

### Connecting via Bluetooth Classic
1. Disconnect the Lightning cable and ensure the keyboard power switch (on the rear edge) is turned **ON** (green indicator visible).
2. Open Windows **Settings** &rarr; **Bluetooth & devices** &rarr; **Add device** &rarr; **Bluetooth**.
3. If the keyboard was previously paired to a Mac, toggle the power switch off and on again to put it in discovery mode.
4. Select **Magic Keyboard with Numeric Keypad** (`VID: 0x004C`, `PID: 0x026C`) from the list of available devices.
5. Windows may prompt you to type a 6-to-8 digit PIN code on the Magic Keyboard followed by the `Return` key.
6. Once paired, MagicBridge instantly activates your active profile.

### Hot-Swapping Between USB and Bluetooth
You can plug or unplug the Lightning cable at any time. MagicBridge listens to Windows PnP device change notifications (`WM_DEVICECHANGE`) and seamlessly transitions between USB and Bluetooth transports without dropping keystrokes or losing your active profile.

---

## 4. System Tray Controls

The MagicBridge background agent lives in the Windows notification area. Right-clicking the tray icon provides quick access to core functions:

- **Enable Remapping (Checked):** Toggle all key remapping on or off. When disabled, the keyboard acts as a generic Windows keyboard.
- **Top Row Mode:**
  - *Media Keys First:* Pressing F7–F12 controls playback and volume. Pressing `Fn + F1–F12` sends standard function keys.
  - *F-Keys First:* Pressing F1–F12 sends standard Windows function keys (ideal for software development and gaming). Pressing `Fn + F1–F12` triggers media controls.
- **Profile Submenu:** Switch between your configured profiles (e.g., *ISO Italian*, *ANSI US*, *Gaming*) with a single click.
- **Battery Tooltip:** Hovering over the tray icon displays the current battery level and connection mode (e.g. `MagicBridge: 88% (Bluetooth) - Active`).
- **Open Settings:** Launches the visual configuration GUI (`magicbridge-settings.exe`).
- **Quit:** Unloads keyboard hooks, restores all keys to Windows defaults, and shuts down the agent cleanly.

---

## 5. The Fn Layer & Keyboard Combos

### Why a User-Mode Trigger Key?
On Windows, the inbox keyboard driver (`kbdhid.sys`) opens the A1843 keyboard exclusively in kernel mode and strips the physical `Fn` usage (`0x00FF:0x0003`) before user-mode software can see it. 

MagicBridge bypasses this limitation by allowing you to designate any convenient key as your **Fn trigger**.
- **Default Fn Trigger:** **Right Option (`VK_RMENU`)**.
- **Alternative Choices:** Right Control, `F19`, or any other key configured in Settings.

### Built-In Fn Navigation Shortcuts
Holding down the Fn trigger key gives you instant access to standard navigation commands:

| Shortcut Combo | Windows Action | Mac Equivalent |
|---|---|---|
| `Fn + Left Arrow (←)` | **Home** | `Cmd + Left Arrow` |
| `Fn + Right Arrow (→)` | **End** | `Cmd + Right Arrow` |
| `Fn + Up Arrow (↑)` | **Page Up** | `Fn + Up Arrow` |
| `Fn + Down Arrow (↓)` | **Page Down** | `Fn + Down Arrow` |
| `Fn + Backspace` | **Forward Delete (⌦)** | `Fn + Delete` |
| `Fn + Enter` | **Insert** | `Insert` |
| `Fn + F1 … F12` | Alternate Top-Row Function | Alternate Top-Row Function |

All Fn combinations can be customized or augmented with your own custom mappings in the Settings GUI.

---

## 6. Top Row & Media Keys

The top row of the A1843 keyboard contains function keys F1 through F12 alongside dedicated multimedia icons.

### Top Row Modes
1. **Media Keys First (Default):**
   - Direct press: triggers media control (Volume, Mute, Play/Pause, Rewind, Fast Forward).
   - `Fn + Key`: sends standard Windows `F1`–`F12` keystrokes.
2. **F-Keys First:**
   - Direct press: sends standard Windows `F1`–`F12` keystrokes.
   - `Fn + Key`: triggers media control.

### Default Media Actions

| Key | Default Media Action | Function Key Alternative |
|---|---|---|
| **F1** | Brightness Down | `F1` |
| **F2** | Brightness Up | `F2` |
| **F3** | **Task View** (`Win + Tab`) | `F3` |
| **F4** | **Windows Start Menu** (`Win`) | `F4` |
| **F7** | Previous Track (`VK_MEDIA_PREV_TRACK`) | `F7` |
| **F8** | Play / Pause (`VK_MEDIA_PLAY_PAUSE`) | `F8` |
| **F9** | Next Track (`VK_MEDIA_NEXT_TRACK`) | `F9` |
| **F10** | Volume Mute (`VK_VOLUME_MUTE`) | `F10` |
| **F11** | Volume Down (`VK_VOLUME_DOWN`) | `F11` |
| **F12** | Volume Up (`VK_VOLUME_UP`) | `F12` |

> **Note on Brightness (F1/F2):** Windows does not provide a universal user-mode desktop brightness API. Brightness control on external desktop monitors depends on monitor DDC/CI support. You can freely remap F1 and F2 to any other key or shortcut in Settings.

---

## 7. Modifier Layout Presets

On Apple keyboards, the lower-left modifier keys are physically laid out as `Control`, `Option`, `Command`. On Windows PC keyboards, they are arranged as `Ctrl`, `Win`, `Alt`.

MagicBridge provides one-click modifier layout presets:

### 1. Windows Layout Preset (Recommended for PC Users)
- Left Command (`Cmd`) &rarr; **Left Alt** (`VK_LMENU`)
- Left Option (`Alt`) &rarr; **Left Windows** (`VK_LWIN`)
- Right Command (`Cmd`) &rarr; **Right Alt** (`VK_RMENU`)
- Right Option (`Alt`) &rarr; **Fn Trigger Key** (or Right Windows)
- Left Control & Right Control remain standard `Ctrl`.

This aligns your muscle memory so `Alt + Tab` and `Win` work exactly where PC keyboards place them.

### 2. Mac Layout Preset
- All modifiers retain their native physical assignments.
- Command sends Windows Key (`VK_LWIN` / `VK_RWIN`).
- Option sends Alt (`VK_LMENU` / `VK_RMENU`).

### 3. Custom Layout
Each modifier can be customized independently in the Settings GUI.

---

## 8. Missing Windows Keys & Special Functions

The A1843 keyboard features an extended numeric keypad and top-row keys F13 through F19. By default, MagicBridge maps these to standard Windows keys missing from Apple hardware:

| Apple Physical Key | MagicBridge Default Mapping | Windows Function |
|---|---|---|
| **F13** | `VK_SNAPSHOT` | **Print Screen** (capture screen) |
| **F14** | `VK_SCROLL` | **Scroll Lock** |
| **F15** | `VK_PAUSE` | **Pause / Break** |
| **F16 – F19** | Unmapped / Custom | Available for custom macros or apps |
| **Clear** | `VK_NUMLOCK` | **Num Lock** (toggles numeric keypad) |
| **Numeric Keypad `=`** | `=` | Standard equals character |

### Custom Actions & Macros
In the Settings GUI, any key from `F13` to `F19` or `Clear` can be assigned to:
1. **Key Remap:** Map to any Windows Virtual Key.
2. **Key Combo (Shortcut):** Trigger combinations like `Ctrl + Shift + Esc` (Task Manager) or `Win + Shift + S` (Snipping Tool).
3. **Application or URL Launcher:** Launch any `.exe`, document, or website URL directly without blocking keyboard input.
4. **Media Command:** Trigger system media actions.

---

## 9. Profiles & Multi-Language Layouts

### Named Profiles
MagicBridge allows you to create and maintain multiple independent **Profiles**. Each profile stores:
- Active physical and language layout (`ISO Italian`, `ANSI US`, `ISO UK`, `ISO German`, `ISO French`, or Custom).
- Modifier preset (Windows, Mac, or Custom).
- Top-row mode (Media first vs. F-keys first).
- Selected Fn trigger key.
- Custom normal-layer and Fn-layer key mappings.

You can switch active profiles instantly from:
- The **System Tray &rarr; Profile** submenu.
- The **Settings GUI** profile dropdown.
- An optional global shortcut hotkey.

### Supported Language Layouts
The Settings GUI provides a faithful, interactive visual representation of the A1843 keyboard matching your regional layout:
- **ANSI (US):** Standard US layout with horizontal Enter key.
- **ISO (Italian):** Standard European ISO layout with vertical L-shaped Enter key, dedicated accented vowels (`à`, `è`, `ì`, `ò`, `ù`), and ISO `§ / ±` key.
- **ISO (UK):** British layout with `£` and `@` placements.
- **ISO (German):** QWERTZ layout with German umlauts (`ä`, `ö`, `ü`, `ß`).
- **ISO (French):** AZERTY layout.

### The ISO Italian AltGr Quirk
On Italian ISO layouts, Windows synthesizes an artificial Left Control key event (`scancode = 0x21D`) approximately 0.8 milliseconds before any `AltGr` (Right Option) press.

MagicBridge includes built-in hardware quirk suppression: whenever Right Option is remapped or used as the Fn trigger key, MagicBridge intercepts and swallows the synthetic `0x21D` scancode, preventing unexpected `Ctrl` triggers in your applications.

---

## 10. Battery Monitoring & Toast Alerts

MagicBridge queries battery levels directly from the Apple hardware:
- **Direct Apple Vendor Telemetry:** Uses Apple Vendor Collection `0xFF00:0x0014`, input report `0x90` (`[0x90, flags, percentage]`) over both USB and Bluetooth.
- **Windows PnP Fallback:** If the HID report is temporarily inaccessible, queries the Windows Bluetooth Battery property.
- **Zero Overhead:** Battery status is polled infrequently (default: every 10 minutes) on a background thread.
- **Tray Tooltip:** Hovering over the tray icon displays the current battery percentage and whether the keyboard is discharging or charging over USB.
- **Low-Battery Toast Alert:** When the battery drops below the configured threshold (default: 15%), Windows displays a notification reminding you to plug in the Lightning cable.

---

## 11. Gaming & Anti-Cheat Compatibility

### Why MagicBridge is Anti-Cheat Safe
Many keyboard remappers use kernel-mode filter drivers (`.sys` files) or inject DLLs into running game processes. These invasive techniques frequently trigger bans by modern anti-cheat systems such as **Riot Vanguard (Valorant)**, **Easy Anti-Cheat (Fortnite, Apex Legends)**, and **BattlEye**.

MagicBridge is engineered with safety as a top priority:
- **Zero Kernel Drivers:** Operates strictly in user mode via standard Win32 input APIs.
- **Zero DLL Injection:** Never injects code or threads into any other process.
- **Zero Memory Modification:** Does not read or write process memory.
- **Self-Injection Marker:** Tags all synthetic keystrokes with a unique extra-info identifier to prevent feedback loops.

### Pause Hotkey for Gaming
If you wish to ensure raw, untouched keyboard input during competitive gameplay:
1. Press the global **Pause Hotkey** (configurable in Settings, or use the System Tray toggle).
2. All remapping immediately disengages.
3. Keystrokes pass through directly to Windows without modification.

---

## 12. Known Limitations & Workarounds

### 1. Windows UIPI (Elevated Administrator Windows)
Windows enforces **User Interface Privilege Isolation (UIPI)**. Under UIPI, applications running with standard user rights cannot send synthetic input into applications running with elevated Administrator rights (such as Task Manager, an elevated Command Prompt, or Regedit).

*Symptom:* Remapped keys or Fn shortcuts do not respond when an elevated Administrator window has active focus.

*Workaround:*
- For 99% of daily desktop tasks (browsers, office suites, editors, games), standard user mode is ideal.
- If you frequently work inside elevated windows, right-click `magicbridge-agent.exe` and select **Run as Administrator**, or configure a scheduled task to run MagicBridge with highest privileges at logon.

### 2. Physical Eject & Physical Fn Stripped in Kernel
As documented in `docs/hardware-findings.md`, Windows' built-in driver `kbdhid.sys` locks the keyboard's primary collection and discards the physical `Fn` usage (`0x00FF:0x0003`) and `Eject` key (`0x000C:0x00B8`) before they reach user mode.
- *Solution:* MagicBridge uses the user-configurable trigger key (default: Right Option) for the Fn layer, and maps `F13`–`F15` to standard missing keys.

### 3. Bluetooth Reconnection Latency
When the keyboard has been idle for several minutes, the Bluetooth radio enters a low-power sleep state. The first keypress wakes the hardware, which can introduce a small latency of ~50–100 ms. Once awake, MagicBridge processes keystrokes in under **0.1 ms**.

---

## 13. Uninstallation

### Portable Clean-Up
If you extracted MagicBridge from a zip archive without running the installer:
1. Right-click the MagicBridge tray icon and click **Quit**.
2. If autostart was enabled, open **Settings &rarr; General** and uncheck *Start with Windows* before quitting (or delete the `MagicBridge` value in `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`).
3. Delete the folder where you extracted MagicBridge.
4. (Optional) Delete your saved profiles and configuration located at `%APPDATA%\MagicBridge`.

### Automated Uninstaller
MagicBridge can be uninstalled cleanly at any time:
1. **Via Windows Settings / Control Panel:**
   Open Windows **Settings &rarr; Apps &rarr; Installed apps**, find **MagicBridge (Apple Magic Keyboard for Windows)**, and click **Uninstall**.
2. **Via PowerShell Script:**
   Open PowerShell and run:
   ```powershell
   .\uninstall.ps1
   ```
   Or for an automated silent purge:
   ```powershell
   .\uninstall.ps1 -Silent -PurgeData
   ```
The uninstaller terminates running processes, removes the autostart registry entry, cleans Start Menu and Desktop shortcuts, removes the Windows Add/Remove Programs registry entry, and removes application binaries. User profiles and settings in `%APPDATA%\MagicBridge` can be preserved or purged.

---

*MagicBridge is free and open-source software licensed under the MIT License.*
