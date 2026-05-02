# Codex Pets Windows Electron Helper

This helper renders the desktop pet overlay used by the WSL-hosted Codex CLI.
It intentionally mirrors the Codex desktop app overlay: transparent frameless
window, app pet spritesheet timing, activity card, collapse control, and badge.

Build it from Windows PowerShell in this directory:

```powershell
npm ci
npm run build
```

For development, `npm start` also works from Windows. The Rust pets bridge
prefers helpers in this order:

1. `pets-windows/electron/dist/codex-pets-windows.exe`
2. `pets-windows/electron/dist/win-unpacked/codex-pets-windows.exe`
3. `pets-windows/electron/node_modules/.bin/electron.cmd pets-windows/electron`
4. `target/x86_64-pc-windows-msvc/<profile>/codex-pets-windows.exe`

Run the Node install/build from Windows, not WSL, if you want the dev `.cmd`
launcher path to be available to PowerShell.
