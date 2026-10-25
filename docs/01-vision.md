# 01 — Vision

> Status: **DRAFT** — under definition, not yet approved.
> Working name: **MagicBridge** (placeholder, can be renamed).

## Problem

The Apple Magic Keyboard with Numeric Keypad (**model A1843**) works on Windows 10 only as a
generic keyboard. Several keys and behaviours are missing or wrong:

- The **Fn** key does nothing, so Fn combinations (Fn+arrows, Fn+Backspace, …) are unavailable.
- The **top row** media functions (volume, play/pause, brightness) don't work reliably, and there is no way
  to choose between "media keys" and "F1–F12".
- **Cmd / Option** sit where Windows expects **Alt / Win**.
- Windows keys such as **Print Screen, Insert, Pause** are missing; the **Eject** key does nothing.
- **F13–F19** and **Clear** are recognized but do nothing useful.
- Windows doesn't show the keyboard's **battery level**.

Existing solutions are paid, heavy, outdated (Boot Camp drivers), or generic remappers that don't
understand the Apple-specific keys.

## Goal

A **tiny, invisible, native** Windows utility that makes the A1843 behave like a first-class
Windows keyboard, with a small settings GUI that has a visual keyboard map.

## Target user

The project owner: a single user on Windows 10 64-bit who games and works on the same PC and
connects the keyboard over **both Bluetooth and USB (Lightning cable)**.

## Guiding principles (in priority order)

1. **Never hurt performance.** Zero CPU when idle, a few MB of RAM, no noticeable input latency.
2. **Never interfere with other apps or games.** No DLL injection, no kernel driver, easy to pause.
3. **Just works.** Starts with Windows, survives sleep/resume and Bluetooth reconnects.
4. **Simple to configure.** All settings in one small GUI with a visual keyboard.

## Non-goals (v1)

- Supporting other Apple keyboards or trackpads. The architecture shouldn't rule them out, though.
- macOS-style features beyond the keyboard: gestures, Touch Bar, etc.
- Kernel-mode drivers or filter drivers.
- Windows 7/8 or 32-bit support. Windows 11 is a nice-to-have, not a requirement.
- Cloud sync, accounts, telemetry. **No network access at all.**
