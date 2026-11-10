# Changelog

## 0.3.0 (2026-11-10)

- **Simple view / Advanced view**: a toggle (remembered) that hides PID, protocol, memory and source, and shows a plain-words label of what holds the port: dev web server (Vite, React…), Node.js app, Python app, Java app, .NET app, database, Docker container, program running in WSL2, system process, other program. The Kill button becomes "Free port". The label comes from a new `kind` field computed by the backend from the process name and command line (display only; no Kill decision depends on it).
- **Toasts**: a discreet notification at the bottom right confirms (or reports the error of) every Kill and Smart Kill, for example "Port 5173 freed (node.exe stopped).".
- **Row fade-out**: the row of a stopped process fades out before the list refreshes.
- New module `toast.js`; 15 Rust tests.

## 0.2.0 (2026-11-10)

- **FR / EN language selector** (2026-11-09): the interface, the backend error messages and the system tray menu follow the chosen language. The choice is remembered; it defaults to the system language.
- **Front end split into ES modules** (2026-11-09): the single `main.js` became `state.js`, `render.js`, `kill.js`, `tree-dialog.js`, `i18n.js` and `main.js`. Still plain JavaScript on purpose (no framework). No behavior change.
- **Clearer Hyper-V banner** (2026-11-10).
- **Documentation**: English README by default (French in `README.fr.md`), English demo GIF and screenshots, English issue templates.

## 0.1.0 (2026-11-09)

- First release: listening TCP/UDP ports, one-click Kill with confirmation, Smart Kill (process tree with `npm` / `yarn` / `pnpm` / `nodemon` launchers), guardrails for system processes, WSL2 and Docker source mapping, Hyper-V excluded port ranges diagnostics, system tray.
