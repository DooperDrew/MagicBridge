# Security Policy for MagicBridge

The **MagicBridge** team takes software integrity, anti-cheat safety, and user security seriously.

---

## 1. Supported Versions

Security updates and critical patches are released for the following versions:

| Version | Supported | Notes |
|---|---|---|
| `v0.2.x` | ✅ Yes | Current stable release branch |
| `v0.1.x` | ❌ No | Please upgrade to `v0.2.1` |

---

## 2. Core Security & Safety Guarantees

MagicBridge is engineered under strict non-functional security constraints:

1. **Anti-Cheat Safe (NFR-S1):** Strictly **no kernel-mode drivers (`.sys`)**, no driver signing bypasses, and **no DLL injection** into external processes. MagicBridge operates purely in user-mode via standard `WH_KEYBOARD_LL` input hooks and `SendInput`.
2. **Infinite Loop Prevention:** All synthetic keystrokes injected by MagicBridge carry a unique, sentinel `dwExtraInfo` marker (`MAGICBRIDGE_EXTRA_INFO`). The input hook immediately passes through any input carrying this tag, preventing input amplification or injection recursion.
3. **Emergency Key Release (NFR-S3):** All pressed synthetic keys are tracked and automatically released upon pause, profile switch, disconnect, or unexpected daemon exit, preventing stuck modifier keys.
4. **Offline & Privacy-Preserving:** MagicBridge makes **zero network connections**, runs no telemetry, collects no keystrokes, and requires no cloud services.

---

## 3. Reporting a Vulnerability

If you discover a potential security vulnerability or safety issue in MagicBridge:
- Please report it via [GitHub Private Vulnerability Reporting](https://github.com/DooperDrew/magicbridge/security/advisories/new).
- Do not open a public issue for zero-day vulnerabilities.
- We will acknowledge receipt within 48 hours and coordinate a timely fix and advisory.
