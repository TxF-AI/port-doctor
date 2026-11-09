# CLAUDE.md - Mémoire & Directives du Projet PortDoctor

## Vue d'ensemble
- **Projet :** PortDoctor (libère les ports bloqués, erreur `EADDRINUSE`, inspecte WSL2/Docker et les plages de ports exclues par Hyper-V). Utilitaire léger dans la zone de notification (System Tray) de Windows.
- **Stack :** Rust (backend) + Tauri v2 (tray) + Vite + JavaScript vanilla + Tailwind CSS v4. Thème « HUD cyberpunk » repris de DevEnv Doctor (`--bg #090a0f`, accents réservés aux statuts).
- **Version courante :** v0.1.0 (dépôt local, non publié). La version vient de `src-tauri/tauri.conf.json` : la garder égale à `Cargo.toml` et `package.json`.
- **Règles produit :** lecture seule par défaut ; seule écriture = arrêt d'un processus, toujours après confirmation visuelle ; processus système protégés ; rien n'est envoyé sur Internet.

## Commandes clés
Gestionnaire de paquets : **npm**. Dans un shell déjà ouvert : `$env:Path += ";$env:USERPROFILE\.cargo\bin"`.
- Installer : `npm install`
- Dev : `npm run tauri dev` (1re compilation Rust longue)
- Tests Rust : `cd src-tauri && cargo test`
- Lint : `cd src-tauri && cargo clippy --all-targets` (0 avertissement attendu)
- Exécutable : `npm run tauri build -- --no-bundle` → `src-tauri\target\release\port-doctor.exe`
- Backend : `src-tauri/src/models.rs` (types), `ports.rs` (scan, classification, garde-fous, Smart Kill `plan_tree`/`kill_tree`, distros WSL2, Hyper-V), `lib.rs` (commandes `get_active_ports`, `kill_port_process`, `kill_process_tree`, `preview_kill_tree`, `get_excluded_ranges`, `get_backend_memory`, tray).
- Langues (FR/EN) : src/i18n.js (dictionnaires, 	()), sélecteur #lang, persistance localStorage, commande set_language (backend : i18n.rs, macro 	r!(fr, en), menu du tray mis à jour). Tout nouveau texte visible = une clé dans les deux langues ; tout message d'erreur Rust = 	r!. Les tests qui lisent un message prennent LANG_LOCK.
- Front : JS vanilla volontaire (exe ~4 Mo, zéro dépendance), découpé en modules ES : `state.js` (état, utilitaires, `visible()`), `render.js` (tableau, stats, bannière, statut), `kill.js` (Kill en 2 temps), `tree-dialog.js` (modale Smart Kill), `i18n.js`, `main.js` (init, refresh, langue). `refresh` est injecté dans `initKill`/`initTreeDialog` (pas d'import circulaire). Pas de framework : ne pas en ajouter sans raison.
- README : `README.md` = anglais (défaut GitHub), `README.fr.md` = français ; médias EN dans `assets/*.gif|png`, FR dans `assets/*.fr.*`.
- Smart Kill : parents arrêtés seulement si lanceurs (`npm`/`yarn`/`pnpm`/`nodemon`, ou `node`/`cmd` dont la ligne de commande les cite) ; jamais PortDoctor ni ses ancêtres ; un processus protégé dans l'arbre = refus total.
- Publication : `gh repo create TxF-AI/port-doctor --public`, tag annoté `v0.1.0`, `gh release create v0.1.0 <exe> --verify-tag --notes-file <fichier>` avec SHA-256.

## Conventions Git
- Commits conventionnels (`feat:`, `fix:`, `docs:`, `refactor:`), en français.
- Auteur : `TxF-AI <206823544+TxF-AI@users.noreply.github.com>`.
- Aucune donnée personnelle dans les fichiers suivis (nom d'utilisateur Windows, e-mail) : vérifier avec `git grep -i -E "$env:USERNAME|gmail"`.

## Memory Log
*Mise à jour obligatoire : à chaque commande échouée, bug complexe (> 2 essais) ou comportement inattendu Windows/Rust/Tauri, consigner le problème, la solution et la règle à suivre. Relire ce fichier avant toute tâche lourde.*

Format : **Problème** / **Solution** / **Règle**.

### Smart Kill, Tailwind et dialog
- **Problème :** `sysinfo::Process::kill()` renvoie `false` pour un parent qui vient de s'arrêter seul (son enfant tué) : faux « accès refusé ». **Solution :** revérifier l'existence du PID (`still_running`) avant de conclure. **Règle :** `kill() == false` ne prouve pas un refus.
- **Problème :** `<dialog>` collé en haut à gauche. **Cause :** le preflight Tailwind v4 met `margin: 0`. **Solution :** `dialog { margin: auto; }`.
- **Règle de test :** valider l'UI réelle avec UI Automation (FindAll Button « Kill », filtrer la ligne par son texte, `InvokePattern`), en ne lançant et n'arrêtant que ses propres processus de test.
- **Problème :** `wsl.exe` sort de l'UTF-16 LE. **Solution :** `--list --running --quiet`, décodage `as_chunks::<2>()`, cache 20 s.

### Hérité de DevEnv Doctor (même machine, même stack)
- **Problème :** `cargo` introuvable dans un shell déjà ouvert. **Solution :** `$env:Path += ";$env:USERPROFILE\.cargo\bin"`. **Règle :** vérifier `cargo --version` avant toute tâche Rust.
- **Problème :** `sed` mange les `\` d'un chemin Windows. **Règle :** utiliser l'outil Edit, jamais `sed`, pour du texte contenant des backslashes.
- **Problème :** l'outil Write refuse un fichier modifié ailleurs. **Règle :** Read avant Write.
- **Problème :** captures de la fenêtre. **Règle :** PrintWindow avec un hook temporaire `TEMP-VALIDATION` retiré ensuite (`grep -rc TEMP-VALIDATION src/` = 0) ; masquer le nom d'utilisateur Windows.
