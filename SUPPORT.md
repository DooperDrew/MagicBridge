# Getting Support for MagicBridge ⌨️ 🌉

Welcome to the **MagicBridge** support guide! Whether you need help setting up your Apple Magic Keyboard on Windows, encountered unexpected hardware behavior, or want to submit feedback, here is where to find answers:

---

## 1. Quick Documentation Links

Before opening an issue, check if your question is covered in our documentation:

- 📖 **[User Guide & Manual](docs/user-guide.md)** — Step-by-step pairing instructions, setup wizard guide, modifier key presets, top row media keys, and troubleshooting.
- 🪄 **[Installation & Setup Wizard](scripts/setup-wizard.ps1)** — Zero-elevation guided installer for Windows 10/11.
- 🧪 **[Hardware QA Verification Matrix](docs/06-qa-and-validation.md)** — Hardware test protocols for Apple Magic Keyboard (Model A1843).
- 📦 **[Package Distribution & WinGet](docs/release-methodology.md)** — Installation via WinGet, Scoop, or portable ZIP.
- 🤝 **[Community & Ecosystem Roadmap](docs/08-community-and-growth-strategy.md)** — Multi-device expansion plan (Trackpad, Mouse, Compact keyboard).

---

## 2. Common Questions & Troubleshooting

### Q: Why does the physical Fn key not work?
**A:** Windows inbox kernel driver `kbdhid.sys` intercepts and suppresses raw Apple Fn key events before user-mode applications can see them. MagicBridge provides user-configurable trigger keys (default: **Right Option / AltGr**, Right Control, or Caps Lock) to activate the Fn layer seamlessly.

### Q: Do I need administrator rights to install MagicBridge?
**A:** **No.** MagicBridge is designed to run entirely in user space. The guided setup wizard installs into `%LOCALAPPDATA%\Programs\MagicBridge` without requiring UAC elevation.

### Q: Does MagicBridge trigger game anti-cheat bans?
**A:** **No.** MagicBridge uses standard Windows user-mode hooks (`WH_KEYBOARD_LL`). It contains **no kernel drivers (`.sys`)** and performs **no DLL injection**. In addition, the tray icon and hotkey allow you to pause remappings instantly while gaming.

---

## 3. How to Report Issues & Submit QA Reports

- **Bug Reports:** Open a [Bug Report](https://github.com/DooperDrew/magicbridge/issues/new?template=bug_report.md) with your Windows build version and whether your keyboard is connected via Lightning USB or Bluetooth.
- **Hardware QA Submissions:** Use the **🧪 Hardware QA** tab in `magicbridge-settings` to verify your physical keys, click **Export QA Log**, and paste it into a [Hardware QA Report](https://github.com/DooperDrew/magicbridge/issues/new?template=hardware_qa_report.md).
- **Feature Requests & Ideas:** Submit a [Feature Request](https://github.com/DooperDrew/magicbridge/issues/new?template=feature_request.md).
