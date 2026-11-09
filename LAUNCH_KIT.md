# LAUNCH_KIT — PortDoctor v0.1.0

Textes prêts à publier. Avant de poster : relire, adapter le ton, joindre un GIF ou une capture (`assets/ui-preview.png`, `assets/ui-smartkill.png`) et **respecter les règles d'auto-promotion de chaque communauté** (flair « Show & Tell » / « Project », pas de multi-posts le même jour, répondre aux commentaires).

- Repo : https://github.com/TxF-AI/port-doctor
- Release : https://github.com/TxF-AI/port-doctor/releases/tag/v0.1.0 (`port-doctor.exe`, 4,3 Mo)
- SHA-256 : `1D541C84B0D1A551EA1D148A23CCEAA27BDE7AB382F66869A30D26DF19972C30`

> Chiffres à ne pas gonfler : l'exécutable fait 4,3 Mo ; la mémoire utilisée en release n'est pas mesurée (l'interface passe par WebView2, pas par un Chromium embarqué). Ne pas annoncer « < 5 Mo de RAM » sans l'avoir mesuré. Les captures du README utilisent des données de démonstration : le dire si on te le demande.

---

## 1. Reddit — Show & Tell

**Subreddits :** `r/rust` (flair *project*), `r/windows`, `r/programming` (lien vers le repo), `r/CoolGithubProjects`. Un subreddit par jour.

**Titre (au choix)**
- `Tired of "EADDRINUSE: port 3000 already in use" and hunting zombie node.exe, so I built a tiny Rust/Tauri tray tool that frees the port in one click`
- `I got sick of netstat | findstr + taskkill every time a dev server hangs — PortDoctor (Rust + Tauri v2, 4.3 MB)`

**Corps**

Hi all,

You know the drill: `Error: listen EADDRINUSE: address already in use :::3000`, then `netstat -ano | findstr :3000`, then `taskkill /F /PID …`, then discovering the real culprit was the `npm` parent that respawns the `node.exe`. And sometimes the port isn't even taken: Windows/Hyper-V reserved a range that includes it.

So I built **PortDoctor**, a small Windows tray app that lists what is listening and frees ports safely.

**What it does**
- **Live port list** (TCP listeners + UDP sockets): PID, process, path, memory, addresses; instant filter and a "Dev ports" view (3000, 5173, 8080, 5432, 6379, 27017…).
- **One-click Kill** with a visual confirmation.
- **Smart Kill**: if the process has children or an `npm`/`yarn`/`pnpm`/`nodemon` launcher above it, a modal shows the exact tree (parents / port owner / children) before stopping it. Parents are opt-in.
- **Guardrails**: `System`, `svchost`, `lsass`… are locked; a tree containing a protected process is refused entirely; it never touches itself, its ancestors, or an ordinary terminal.
- **WSL2 / Docker mapping**: ports relayed by `wslrelay.exe` are labelled with the running distro (`WSL2 (Ubuntu)`), containers as `Docker`.
- **Hyper-V excluded ranges**: warns when a dev port (say 5000) falls inside a range Windows reserved (`netsh interface ipv4 show excludedportrange`).

**Why Rust + Tauri v2**: a single portable 4.3 MB exe, native socket tables (`netstat2`) and process tree handling (`sysinfo`), no Electron, lives in the system tray. 13 tests, some killing real child processes. MIT, nothing leaves your machine.

**Download / verify** (unsigned, so SmartScreen will warn; compare the hash):
https://github.com/TxF-AI/port-doctor/releases/tag/v0.1.0
`SHA-256: 1D541C84B0D1A551EA1D148A23CCEAA27BDE7AB382F66869A30D26DF19972C30`

It's v0.1.0 — I'd love **feedback**: processes it should (not) kill, WSL2/Docker setups where the label is wrong, what you'd want next. Issues and PRs welcome.

---

## 2. Hacker News — Show HN

**Titre**
`Show HN: PortDoctor – Free blocked ports and kill process trees on Windows (Rust/Tauri)`

**URL :** https://github.com/TxF-AI/port-doctor

**Premier commentaire**

Hi HN, author here.

PortDoctor is a tray utility for the classic `EADDRINUSE` problem on Windows: it lists listening TCP/UDP sockets with their owning process, and kills the owner (or the whole dev-server tree) in one click.

Some choices worth discussing:
- **Tree-aware kill.** Killing the `node.exe` that holds port 3000 often does nothing, because an `npm`/`nodemon` parent respawns it. Smart Kill computes the tree (children, plus parents *only if* they look like dev launchers) and shows it before acting. Parents are stopped first so nothing respawns.
- **Guardrails in the backend, not just the UI.** The backend re-scans before every kill and refuses a PID that no longer owns a port (PID reuse), protected system processes, itself and its own ancestors (it is often started by `npm run tauri dev`), and any tree containing a protected process.
- **Honest labels.** `wslrelay.exe` hides which distro owns a port; with several distros running, the label lists them all rather than guessing.
- **Hyper-V excluded port ranges** are parsed from `netsh` locale-independently (only "start end [*]" lines) to warn when a dev port is reserved by Windows.
- **Rust + Tauri v2, no Electron**: `netstat2` for sockets, `sysinfo` for processes, ~4.3 MB portable exe on the system WebView2. I haven't benchmarked memory, so no number here.
- Tests include spawning real child processes and killing them. MIT, no network access.

Caveats: Windows only, unsigned exe (SmartScreen warning; SHA-256 in the release), killing an elevated process requires running PortDoctor as admin.

Feedback welcome, especially on what a tool like this should refuse to kill.

---

## 3. X (Twitter) & LinkedIn

**Média à joindre :** GIF de 10–15 s : serveur de dev qui plante en `EADDRINUSE` → clic sur l'icône du tray → filtre « Dev ports » → Kill → modale Smart Kill → confirmer → relancer le serveur. Ou à défaut `assets/ui-preview.png` + `assets/ui-smartkill.png`.

**X (≤ 280 caractères)**

```
EADDRINUSE again? 😤

PortDoctor 🔌 lives in your Windows tray (Rust + Tauri v2, 4.3 MB):
💥 free a port in 1 click
🌳 Smart Kill of the whole npm → node tree
🐧 WSL2 / Docker labels
⚠️ Hyper-V reserved ranges

MIT 👉 github.com/TxF-AI/port-doctor
#RustLang #Tauri #BuildInPublic #OpenSource
```

**X (thread, 2ᵉ tweet)**
```
Guardrails matter: svchost/lsass/System are locked, a tree containing a protected process is refused, and it never kills itself or its ancestors. Tests even spawn and kill real processes. Feedback welcome 🙏
```

**LinkedIn (FR)**

```
🔌 « EADDRINUSE : address already in use :::3000 » — j'en avais marre, alors j'ai codé PortDoctor.

Un serveur de dev mal arrêté, un node.exe zombie, un parent npm qui le relance, un port réservé par Hyper-V sans le savoir… et dix minutes perdues entre netstat et taskkill.

PortDoctor (v0.1.0), dans la zone de notification de Windows :
🔍 liste des ports TCP/UDP avec processus, chemin, mémoire, filtre « Dev ports »
💥 Kill en un clic, avec confirmation
🌳 Smart Kill : l'arbre npm → vite → node est affiché avant d'être arrêté
🛡️ garde-fous : processus système verrouillés, jamais d'auto-kill
🐧 distribution WSL2 détectée (WSL2 (Ubuntu)), conteneurs Docker repérés
⚠️ alerte sur les plages de ports exclues par Hyper-V

Rust + Tauri v2 : un exécutable portable de 4,3 Mo, rien n'est envoyé sur Internet, licence MIT.

👉 https://github.com/TxF-AI/port-doctor

Vos retours m'intéressent : quels processus un tel outil devrait-il refuser de tuer ?

#RustLang #Tauri #BuildInPublic #OpenSource #Windows #DevTools
```

**LinkedIn (EN, variante courte)**
```
Built a small Rust/Tauri v2 tray tool for the Windows EADDRINUSE problem: lists listening ports, frees them in one click, Smart Kill for npm→node trees, WSL2/Docker labels, Hyper-V reserved-range alerts. 4.3 MB, MIT, local only.
github.com/TxF-AI/port-doctor — feedback very welcome!
#RustLang #Tauri #BuildInPublic #OpenSource
```

---

## 4. Discord — #showcase

**Salons :** Discord officiel Tauri (`#showcase`), DevFR (salon projets), serveurs Rust FR/EN. Lire les règles avant de poster.

**Message (EN)**
```
🔌 **PortDoctor** — a Tauri v2 tray app that frees blocked ports on Windows
Lists listening TCP/UDP ports with their process, one-click Kill, Smart Kill of the whole npm→node tree (shown in a modal first), WSL2 distro / Docker labels, Hyper-V reserved-range alerts. Protected system processes are locked. 4.3 MB portable exe, MIT, 100% local.
🔗 https://github.com/TxF-AI/port-doctor
Feedback on the UI and on what it should refuse to kill is very welcome! 🙏
```

**Message (FR, DevFR)**
```
🔌 **PortDoctor** — libère les ports bloqués sur Windows (Rust + Tauri v2, System Tray)
Liste des ports TCP/UDP avec leur processus, Kill en 1 clic, Smart Kill de l'arbre npm→node (modale de confirmation), distro WSL2 / Docker repérés, alerte plages Hyper-V. Processus système verrouillés. Exe portable 4,3 Mo, MIT, 100 % local.
🔗 https://github.com/TxF-AI/port-doctor
Vos retours sont les bienvenus (UI, ce qu'il ne devrait jamais tuer) !
```
