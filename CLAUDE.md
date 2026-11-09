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
- Backend : `src-tauri/src/models.rs` (types), `ports.rs` (scan, classification, garde-fous, Hyper-V), `lib.rs` (commandes `get_active_ports`, `kill_port_process`, `get_excluded_ranges`, tray).

## Conventions Git
- Commits conventionnels (`feat:`, `fix:`, `docs:`, `refactor:`), en français.
- Auteur : `TxF-AI <206823544+TxF-AI@users.noreply.github.com>`.
- Aucune donnée personnelle dans les fichiers suivis (nom d'utilisateur Windows, e-mail) : vérifier avec `git grep -i -E "$env:USERNAME|gmail"`.

## Memory Log
*Mise à jour obligatoire : à chaque commande échouée, bug complexe (> 2 essais) ou comportement inattendu Windows/Rust/Tauri, consigner le problème, la solution et la règle à suivre. Relire ce fichier avant toute tâche lourde.*

Format : **Problème** / **Solution** / **Règle**.

### Hérité de DevEnv Doctor (même machine, même stack)
- **Problème :** `cargo` introuvable dans un shell déjà ouvert. **Solution :** `$env:Path += ";$env:USERPROFILE\.cargo\bin"`. **Règle :** vérifier `cargo --version` avant toute tâche Rust.
- **Problème :** `sed` mange les `\` d'un chemin Windows. **Règle :** utiliser l'outil Edit, jamais `sed`, pour du texte contenant des backslashes.
- **Problème :** l'outil Write refuse un fichier modifié ailleurs. **Règle :** Read avant Write.
- **Problème :** captures de la fenêtre. **Règle :** PrintWindow avec un hook temporaire `TEMP-VALIDATION` retiré ensuite (`grep -c TEMP-VALIDATION src/main.js` = 0) ; masquer le nom d'utilisateur Windows.
