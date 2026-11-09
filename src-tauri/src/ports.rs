use std::collections::BTreeMap;
use std::collections::BTreeSet;

use netstat2::{AddressFamilyFlags, ProtocolFlags, ProtocolSocketInfo, TcpState, get_sockets_info};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

use crate::models::{ExcludedRange, KillOutcome, PortEntry, TreeMember};

/// Ports de développement courants, mis en avant par l'interface.
pub const DEV_PORTS: [u16; 10] = [
    3000, 3001, 4200, 5173, 8080, 8000, 5432, 6379, 27017, 3306,
];

/// Processus que PortDoctor refuse toujours de terminer (comparaison en minuscules).
const PROTECTED_NAMES: [&str; 11] = [
    "system",
    "system idle process",
    "registry",
    "smss.exe",
    "csrss.exe",
    "wininit.exe",
    "winlogon.exe",
    "services.exe",
    "lsass.exe",
    "svchost.exe",
    "dwm.exe",
];

pub fn is_dev_port(port: u16) -> bool {
    DEV_PORTS.contains(&port)
}

/// PID 0 (Idle) et 4 (System) sont toujours protégés, quel que soit le nom lu.
pub fn is_protected(pid: u32, name: &str) -> bool {
    pid == 0 || pid == 4 || PROTECTED_NAMES.contains(&name.to_lowercase().as_str())
}

/// Étiquette de provenance : un port ouvert depuis une VM WSL2 ou un conteneur est
/// relayé sur l'hôte par `wslrelay.exe` / un proxy Docker.
pub fn classify_source(process_name: &str) -> &'static str {
    let n = process_name.to_lowercase();
    if n == "wslrelay.exe" || n == "wslhost.exe" || n == "wsl.exe" {
        "WSL2"
    } else if n.contains("docker") || n.contains("vpnkit") {
        "Docker"
    } else {
        "Host"
    }
}

/// Socket brut avant fusion : (protocole, port, adresse, pid).
type RawSocket = (&'static str, u16, String, u32);

fn raw_listeners() -> Result<Vec<RawSocket>, String> {
    let sockets = get_sockets_info(
        AddressFamilyFlags::IPV4 | AddressFamilyFlags::IPV6,
        ProtocolFlags::TCP | ProtocolFlags::UDP,
    )
    .map_err(|e| tr!(format!("Lecture des sockets impossible : {e}"), format!("Cannot read sockets: {e}")))?;

    let mut out = Vec::new();
    for s in sockets {
        let (proto, addr, port) = match s.protocol_socket_info {
            ProtocolSocketInfo::Tcp(t) if t.state == TcpState::Listen => {
                ("TCP", t.local_addr.to_string(), t.local_port)
            }
            // UDP n'a pas d'état : tout socket lié est « à l'écoute ».
            ProtocolSocketInfo::Udp(u) => ("UDP", u.local_addr.to_string(), u.local_port),
            _ => continue,
        };
        for pid in s.associated_pids {
            out.push((proto, port, addr.clone(), pid));
        }
    }
    Ok(out)
}

/// Fusionne les lignes `(proto, port, pid)` identiques (IPv4 + IPv6 → une seule ligne).
fn merge(raw: Vec<RawSocket>, sys: &System) -> Vec<PortEntry> {
    let mut groups: BTreeMap<(u16, &'static str, u32), BTreeSet<String>> = BTreeMap::new();
    for (proto, port, addr, pid) in raw {
        groups.entry((port, proto, pid)).or_default().insert(addr);
    }

    groups
        .into_iter()
        .map(|((port, proto, pid), addrs)| {
            let process = sys.process(Pid::from_u32(pid));
            let name = match (pid, process) {
                (0, _) => "System Idle Process".to_string(),
                (_, Some(p)) => p.name().to_string_lossy().into_owned(),
                (_, None) => tr!("Inconnu", "Unknown").to_string(),
            };
            let parent = process.and_then(|p| p.parent());
            let parent_name = parent
                .and_then(|pp| sys.process(pp))
                .map(|p| p.name().to_string_lossy().into_owned());
            PortEntry {
                port,
                proto: proto.to_string(),
                addresses: addrs.into_iter().collect(),
                pid,
                protected: is_protected(pid, &name),
                source: classify_source(&name).to_string(),
                is_dev_port: is_dev_port(port),
                exe_path: process
                    .and_then(|p| p.exe())
                    .map(|p| p.to_string_lossy().into_owned()),
                memory_bytes: process.map_or(0, |p| p.memory()),
                parent_pid: parent.map(|p| p.as_u32()),
                parent_name,
                process_name: name,
            }
        })
        .collect()
}

fn fresh_system() -> System {
    let mut sys = System::new();
    sys.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing()
            .with_memory()
            .with_exe(UpdateKind::OnlyIfNotSet)
            .with_cmd(UpdateKind::OnlyIfNotSet),
    );
    sys
}

/// Liste les ports TCP en écoute et les sockets UDP liés, avec leur processus.
pub fn scan() -> Result<Vec<PortEntry>, String> {
    let raw = raw_listeners()?;
    let sys = fresh_system();
    let mut entries = merge(raw, &sys);
    if entries.iter().any(|e| e.source == "WSL2") {
        let label = wsl_label(&running_distros());
        for e in entries.iter_mut().filter(|e| e.source == "WSL2") {
            e.source = label.clone();
        }
    }
    Ok(entries)
}

// ───────────────────────── WSL2 : distributions en cours d'exécution ─────────────────────────

/// Décode la sortie de `wsl.exe` (UTF-16 LE sur la plupart des versions, UTF-8 sinon)
/// et renvoie les noms de distributions, sans les distributions internes de Docker Desktop.
pub fn parse_wsl_list(bytes: &[u8]) -> Vec<String> {
    let text = if bytes.len() >= 2 && bytes.contains(&0) {
        let (pairs, _) = bytes.as_chunks::<2>();
        let units: Vec<u16> = pairs.iter().map(|c| u16::from_le_bytes(*c)).collect();
        String::from_utf16_lossy(&units)
    } else {
        String::from_utf8_lossy(bytes).into_owned()
    };
    text.lines()
        .map(|l| l.trim().trim_start_matches('\u{feff}').trim())
        .map(|l| l.split(" (").next().unwrap_or(l).trim())
        .filter(|l| !l.is_empty() && !l.to_lowercase().starts_with("docker-desktop"))
        .map(str::to_string)
        .collect()
}

/// `WSL2 (Ubuntu)`, `WSL2 (Ubuntu, Debian)` si plusieurs sont actives (le port ne dit pas
/// laquelle l'ouvre), ou `WSL2` seul si la liste est indisponible.
pub fn wsl_label(distros: &[String]) -> String {
    if distros.is_empty() {
        "WSL2".to_string()
    } else {
        format!("WSL2 ({})", distros.join(", "))
    }
}

static WSL_CACHE: Mutex<Option<(Instant, Vec<String>)>> = Mutex::new(None);

/// Distributions actives (`wsl.exe --list --running --quiet`), mises en cache 20 s :
/// l'interface rafraîchit toutes les 3 s. Lister n'allume pas la VM.
fn running_distros() -> Vec<String> {
    use std::os::windows::process::CommandExt;
    if let Ok(g) = WSL_CACHE.lock()
        && let Some((t, v)) = g.as_ref()
        && t.elapsed() < Duration::from_secs(20)
    {
        return v.clone();
    }
    let list = std::process::Command::new("wsl.exe")
        .args(["--list", "--running", "--quiet"])
        .creation_flags(0x0800_0000)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| parse_wsl_list(&o.stdout))
        .unwrap_or_default();
    if let Ok(mut g) = WSL_CACHE.lock() {
        *g = Some((Instant::now(), list.clone()));
    }
    list
}

// ───────────────────────── Smart Kill : arbre de processus ─────────────────────────

/// Vue minimale d'un processus pour calculer un arbre (testable sans système).
#[derive(Debug, Clone)]
pub struct ProcInfo {
    pub pid: u32,
    pub parent: Option<u32>,
    pub name: String,
    pub cmd: String,
}

fn base_name(name: &str) -> String {
    let n = name.to_lowercase();
    n.strip_suffix(".exe")
        .or_else(|| n.strip_suffix(".cmd"))
        .unwrap_or(&n)
        .to_string()
}

/// Un parent n'est arrêté avec son enfant que s'il ressemble à un lanceur de dev
/// (`npm`, `yarn`, `nodemon`…). Un shell ou un terminal ordinaire n'en est jamais un.
pub fn is_launcher(p: &ProcInfo) -> bool {
    const HINTS: [&str; 9] = [
        "npm-cli", "npx-cli", "npm run", "npm start", "npm.cmd", "npx ", "pnpm", "yarn", "nodemon",
    ];
    let base = base_name(&p.name);
    match base.as_str() {
        "npm" | "npx" | "pnpm" | "yarn" | "bun" | "nodemon" => true,
        "node" | "cmd" => {
            let cmd = p.cmd.to_lowercase();
            HINTS.iter().any(|h| cmd.contains(h))
        }
        _ => false,
    }
}

/// Calcule les processus à arrêter : parents lanceurs (si demandé), cible, descendants.
/// Garde-fous appliqués à CHAQUE processus : jamais PortDoctor ni ses ancêtres ; un
/// descendant protégé (svchost, lsass…) fait refuser tout l'arbre.
pub fn plan_tree(
    procs: &[ProcInfo],
    root: u32,
    own_pid: u32,
    include_parents: bool,
) -> Result<Vec<TreeMember>, String> {
    let by_pid = |pid: u32| procs.iter().find(|p| p.pid == pid);
    let target = by_pid(root).ok_or_else(|| tr!(format!("Le PID {root} n'existe plus."), format!("PID {root} no longer exists.")))?;
    if is_protected(target.pid, &target.name) {
        return Err(tr!(format!("{} (PID {root}) est protégé : arrêt refusé.", target.name), format!("{} (PID {root}) is protected: stop refused.", target.name)));
    }

    // PortDoctor et tous ses ancêtres sont intouchables (lancé depuis `npm run tauri dev`, p. ex.).
    let mut safe = vec![own_pid];
    let mut cur = by_pid(own_pid).and_then(|p| p.parent);
    while let Some(pid) = cur {
        if safe.contains(&pid) {
            break;
        }
        safe.push(pid);
        cur = by_pid(pid).and_then(|p| p.parent);
    }
    if safe.contains(&root) {
        return Err(tr!("PortDoctor ne peut pas arrêter son propre arbre de processus.", "PortDoctor cannot stop its own process tree.").into());
    }

    let member = |p: &ProcInfo, role| TreeMember {
        pid: p.pid,
        name: p.name.clone(),
        role,
    };

    // Parents lanceurs, du plus proche au plus lointain, jusqu'au premier non-lanceur.
    let mut parents = Vec::new();
    if include_parents {
        let mut cur = target.parent;
        while let Some(pid) = cur {
            let Some(p) = by_pid(pid) else { break };
            if !is_launcher(p) || is_protected(p.pid, &p.name) || safe.contains(&pid) {
                break;
            }
            parents.push(member(p, "parent"));
            cur = p.parent;
        }
        parents.reverse(); // du plus haut au plus proche
    }

    // Descendants (largeur d'abord) ; cycles impossibles grâce à `seen`.
    let mut children: Vec<TreeMember> = Vec::new();
    let mut seen = vec![root];
    let mut queue = vec![root];
    while let Some(parent) = queue.pop() {
        for p in procs.iter().filter(|p| p.parent == Some(parent)) {
            if seen.contains(&p.pid) || p.pid == p.parent.unwrap_or(0) {
                continue;
            }
            seen.push(p.pid);
            if safe.contains(&p.pid) {
                continue;
            }
            if is_protected(p.pid, &p.name) {
                return Err(tr!(
                    format!("L'arbre contient {} (PID {}), processus protégé : arrêt refusé.", p.name, p.pid),
                    format!("The tree contains {} (PID {}), a protected process: stop refused.", p.name, p.pid)
                ));
            }
            children.push(member(p, "child"));
            queue.push(p.pid);
        }
    }

    let mut plan = parents;
    plan.push(member(target, "target"));
    plan.extend(children);
    Ok(plan)
}

fn snapshot(sys: &System) -> Vec<ProcInfo> {
    sys.processes()
        .iter()
        .map(|(pid, p)| ProcInfo {
            pid: pid.as_u32(),
            parent: p.parent().map(|pp| pp.as_u32()),
            name: p.name().to_string_lossy().into_owned(),
            cmd: p
                .cmd()
                .iter()
                .map(|a| a.to_string_lossy())
                .collect::<Vec<_>>()
                .join(" "),
        })
        .collect()
}

fn still_running(pid: u32) -> bool {
    std::thread::sleep(Duration::from_millis(150));
    let mut sys = System::new();
    let p = Pid::from_u32(pid);
    sys.refresh_processes(ProcessesToUpdate::Some(&[p]), true);
    sys.process(p).is_some()
}

/// Aperçu (lecture seule) de ce qu'un Smart Kill arrêterait, parents lanceurs inclus.
pub fn preview_tree(pid: u32, own_pid: u32) -> Result<Vec<TreeMember>, String> {
    let entries = scan()?;
    guard_kill(pid, own_pid, &entries)?;
    plan_tree(&snapshot(&fresh_system()), pid, own_pid, true)
}

/// Arrête l'arbre d'un processus qui détient un port. Ordre : parents (du haut vers le bas,
/// pour qu'un lanceur ne relance rien), puis descendants du plus profond au plus haut, puis la
/// cible. `Ok(true)` = tout est arrêté ; `Err` liste ce qui a résisté.
pub fn kill_tree(pid: u32, own_pid: u32, include_parents: bool) -> Result<bool, String> {
    let entries = scan()?;
    guard_kill(pid, own_pid, &entries)?;
    let sys = fresh_system();
    let plan = plan_tree(&snapshot(&sys), pid, own_pid, include_parents)?;

    // Descendants : les derniers trouvés sont les plus profonds, on les arrête en premier.
    let of = |role: &'static str| plan.iter().filter(move |m| m.role == role);
    let sequence = of("parent").chain(of("child").rev()).chain(of("target"));

    let mut failed = Vec::new();
    for m in sequence {
        match sys.process(Pid::from_u32(m.pid)) {
            Some(p) if p.kill() => {}
            // kill() est faux aussi quand le processus vient de s'arrêter tout seul
            // (un parent dont l'enfant meurt) : on revérifie avant de conclure à un refus.
            Some(_) if !still_running(m.pid) => {}
            Some(_) => failed.push(format!("{} (PID {})", m.name, m.pid)),
            None => {} // déjà parti avec son parent
        }
    }
    if failed.is_empty() {
        Ok(true)
    } else {
        Err(tr!(
            format!("Accès refusé pour : {}. Relance PortDoctor en administrateur.", failed.join(", ")),
            format!("Access denied for: {}. Relaunch PortDoctor as administrator.", failed.join(", "))
        ))
    }
}

/// Contrôles avant un arrêt : fonction pure, testée sans toucher à un processus.
pub fn guard_kill(pid: u32, own_pid: u32, entries: &[PortEntry]) -> Result<&PortEntry, String> {
    if pid == own_pid {
        return Err(tr!("PortDoctor ne peut pas se terminer lui-même ici.", "PortDoctor cannot stop itself.").into());
    }
    // Le PID doit détenir un port MAINTENANT : évite de tuer un processus qui a réutilisé
    // un PID après le dernier scan affiché.
    let entry = entries
        .iter()
        .find(|e| e.pid == pid)
        .ok_or_else(|| tr!(format!("Le PID {pid} ne détient plus aucun port : rafraîchis la liste."), format!("PID {pid} no longer holds any port: refresh the list.")))?;
    if entry.protected {
        return Err(tr!(
            format!("{} (PID {pid}) est un processus système protégé : arrêt refusé.", entry.process_name),
            format!("{} (PID {pid}) is a protected system process: stop refused.", entry.process_name)
        ));
    }
    Ok(entry)
}

/// Termine le processus qui détient un port (arrêt forcé), après les garde-fous.
pub fn kill_listener(pid: u32, own_pid: u32) -> Result<KillOutcome, String> {
    let entries = scan()?;
    let name = guard_kill(pid, own_pid, &entries)?.process_name.clone();

    let sys = fresh_system();
    let process = sys
        .process(Pid::from_u32(pid))
        .ok_or_else(|| tr!(format!("{name} (PID {pid}) s'est déjà arrêté."), format!("{name} (PID {pid}) has already stopped.")))?;
    if process.kill() {
        Ok(KillOutcome {
            pid,
            process_name: name,
        })
    } else {
        Err(tr!(
            format!("Accès refusé pour {name} (PID {pid}) : processus lancé en administrateur ? Relance PortDoctor en administrateur."),
            format!("Access denied for {name} (PID {pid}): process started as administrator? Relaunch PortDoctor as administrator.")
        ))
    }
}

/// Analyse la sortie de `netsh interface ipv4 show excludedportrange protocol=tcp`.
/// Indépendante de la langue : on ne garde que les lignes « début fin [*] ».
pub fn parse_excluded(output: &str) -> Vec<ExcludedRange> {
    output
        .lines()
        .filter_map(|line| {
            let mut it = line.split_whitespace();
            let start: u16 = it.next()?.parse().ok()?;
            let end: u16 = it.next()?.parse().ok()?;
            let administered = match it.next() {
                None => false,
                Some("*") => true,
                Some(_) => return None,
            };
            (start <= end).then_some(ExcludedRange {
                start,
                end,
                administered,
            })
        })
        .collect()
}

/// Plages de ports TCP réservées par Windows (lecture seule, via netsh).
pub fn excluded_ranges() -> Result<Vec<ExcludedRange>, String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let out = std::process::Command::new("netsh")
        .args([
            "interface",
            "ipv4",
            "show",
            "excludedportrange",
            "protocol=tcp",
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| tr!(format!("netsh indisponible : {e}"), format!("netsh unavailable: {e}")))?;
    if !out.status.success() {
        return Err(tr!("netsh a échoué : plages exclues non disponibles.", "netsh failed: excluded ranges unavailable.").into());
    }
    Ok(parse_excluded(&String::from_utf8_lossy(&out.stdout)))
}

/// Mémoire du processus PortDoctor lui-même (backend Rust, hors WebView).
pub fn own_memory(own_pid: u32) -> u64 {
    let mut sys = System::new();
    sys.refresh_processes(ProcessesToUpdate::Some(&[Pid::from_u32(own_pid)]), true);
    sys.process(Pid::from_u32(own_pid)).map_or(0, |p| p.memory())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use std::sync::Mutex;
    use std::time::{Duration, Instant};

    /// La langue est globale : les tests qui lisent un message la figent le temps du test.
    static LANG_LOCK: Mutex<()> = Mutex::new(());

    fn entry(pid: u32, name: &str) -> PortEntry {
        PortEntry {
            port: 3000,
            proto: "TCP".into(),
            addresses: vec!["127.0.0.1".into()],
            pid,
            process_name: name.into(),
            exe_path: None,
            memory_bytes: 0,
            source: "Host".into(),
            is_dev_port: true,
            protected: is_protected(pid, name),
            parent_pid: None,
            parent_name: None,
        }
    }

    #[test]
    fn dev_ports_are_flagged() {
        for p in DEV_PORTS {
            assert!(is_dev_port(p));
        }
        assert!(!is_dev_port(5000));
    }

    #[test]
    fn protected_by_pid_or_name() {
        assert!(is_protected(4, "n'importe quoi"));
        assert!(is_protected(0, "x"));
        assert!(is_protected(900, "SVCHOST.EXE"));
        assert!(is_protected(900, "lsass.exe"));
        assert!(!is_protected(900, "node.exe"));
    }

    #[test]
    fn source_classification() {
        assert_eq!(classify_source("wslrelay.exe"), "WSL2");
        assert_eq!(classify_source("WSLRelay.exe"), "WSL2");
        assert_eq!(classify_source("com.docker.backend.exe"), "Docker");
        assert_eq!(classify_source("docker-proxy.exe"), "Docker");
        assert_eq!(classify_source("node.exe"), "Host");
    }

    #[test]
    fn guard_rejects_self_unknown_and_protected() {
        let _l = LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        crate::i18n::set_english(false);
        let list = vec![entry(100, "node.exe"), entry(4, "System")];
        assert!(guard_kill(100, 100, &list).is_err()); // soi-même
        assert!(guard_kill(555, 1, &list).is_err()); // PID sans port
        assert!(guard_kill(4, 1, &list).unwrap_err().contains("protégé"));
        assert_eq!(guard_kill(100, 1, &list).unwrap().process_name, "node.exe");
    }

    #[test]
    fn parses_netsh_output_in_any_language() {
        let fr = "\nProtocole ipv4 Plage de exclusion de ports\n\nDe début    Fin\n----------  --------\n      1801        1900\n     50000       50059     *\n\n* - Exclusions administrées.\n";
        let r = parse_excluded(fr);
        assert_eq!(r.len(), 2);
        assert_eq!((r[0].start, r[0].end, r[0].administered), (1801, 1900, false));
        assert_eq!((r[1].start, r[1].end, r[1].administered), (50000, 50059, true));
        assert!(parse_excluded("rien d'utile").is_empty());
        assert!(parse_excluded("  900  100").is_empty()); // début > fin ignoré
    }

    fn pi(pid: u32, parent: Option<u32>, name: &str, cmd: &str) -> ProcInfo {
        ProcInfo { pid, parent, name: name.into(), cmd: cmd.into() }
    }

    /// explorer(1) → cmd(10, "npm run dev") → npm(11) → node(12, port) → node(13) ; PortDoctor = 99.
    fn sample() -> Vec<ProcInfo> {
        vec![
            pi(1, None, "explorer.exe", ""),
            pi(10, Some(1), "cmd.exe", "cmd /c npm run dev"),
            pi(11, Some(10), "node.exe", "node npm-cli.js run dev"),
            pi(12, Some(11), "node.exe", "node vite.js"),
            pi(13, Some(12), "node.exe", "node esbuild"),
            pi(99, Some(1), "port-doctor.exe", ""),
        ]
    }

    fn pids(plan: &[TreeMember]) -> Vec<u32> {
        plan.iter().map(|m| m.pid).collect()
    }

    #[test]
    fn tree_plan_with_and_without_parents() {
        let p = plan_tree(&sample(), 12, 99, true).unwrap();
        assert_eq!(pids(&p), vec![10, 11, 12, 13]); // explorer.exe n'est pas un lanceur : on s'arrête
        assert_eq!(p[0].role, "parent");
        assert_eq!(p[2].role, "target");
        assert_eq!(p[3].role, "child");
        let p = plan_tree(&sample(), 12, 99, false).unwrap();
        assert_eq!(pids(&p), vec![12, 13]);
    }

    #[test]
    fn tree_never_touches_ordinary_shells_or_self() {
        // Parent powershell ordinaire (terminal de l'utilisateur) : jamais arrêté.
        let procs = vec![
            pi(1, None, "explorer.exe", ""),
            pi(20, Some(1), "powershell.exe", "powershell"),
            pi(21, Some(20), "node.exe", "node server.js"),
        ];
        assert_eq!(pids(&plan_tree(&procs, 21, 99, true).unwrap()), vec![21]);
        // PortDoctor lancé par `npm run tauri dev` : ni lui ni ses ancêtres ne sont ciblés.
        let mut dev = sample();
        dev[5] = pi(99, Some(11), "port-doctor.exe", "");
        // Cible 12 : ses parents 10 et 11 sont des ancêtres de PortDoctor → exclus.
        let p = plan_tree(&dev, 12, 99, true).unwrap();
        assert_eq!(pids(&p), vec![12, 13]);
        assert!(plan_tree(&dev, 11, 99, false).is_err()); // ancêtre de PortDoctor
        assert!(plan_tree(&dev, 99, 99, true).is_err());
    }

    #[test]
    fn tree_refuses_protected_target_or_descendant() {
        let _l = LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        crate::i18n::set_english(false);
        let procs = vec![
            pi(1, None, "explorer.exe", ""),
            pi(30, Some(1), "node.exe", "node"),
            pi(31, Some(30), "svchost.exe", ""),
        ];
        assert!(plan_tree(&procs, 30, 99, false).unwrap_err().contains("protégé"));
        assert!(plan_tree(&[pi(900, None, "lsass.exe", "")], 900, 99, true).is_err());
        assert!(plan_tree(&procs, 777, 99, true).is_err()); // PID inconnu
    }

    #[test]
    fn messages_follow_the_selected_language() {
        let _l = LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let list = vec![entry(4, "System")];
        crate::i18n::set_english(true);
        let en = guard_kill(4, 1, &list).unwrap_err();
        let en_missing = guard_kill(555, 1, &list).unwrap_err();
        crate::i18n::set_english(false);
        let fr = guard_kill(4, 1, &list).unwrap_err();
        assert!(en.contains("protected system process"), "{en}");
        assert!(en_missing.contains("no longer holds any port"), "{en_missing}");
        assert!(fr.contains("processus système protégé"), "{fr}");
        assert_ne!(en, fr);
    }

    #[test]
    fn launcher_detection() {
        assert!(is_launcher(&pi(1, None, "npm.cmd", "")));
        assert!(is_launcher(&pi(1, None, "cmd.exe", "cmd /c yarn dev")));
        assert!(!is_launcher(&pi(1, None, "cmd.exe", "cmd")));
        assert!(!is_launcher(&pi(1, None, "node.exe", "node server.js")));
        assert!(!is_launcher(&pi(1, None, "explorer.exe", "")));
    }

    #[test]
    fn wsl_list_parsing_and_label() {
        let utf16: Vec<u8> = "\u{feff}Ubuntu\r\ndocker-desktop\r\nDebian (Default)\r\n"
            .encode_utf16()
            .flat_map(|u| u.to_le_bytes())
            .collect();
        assert_eq!(parse_wsl_list(&utf16), vec!["Ubuntu", "Debian"]);
        assert_eq!(parse_wsl_list(b"Ubuntu-22.04\n"), vec!["Ubuntu-22.04"]);
        assert!(parse_wsl_list(b"").is_empty());
        assert_eq!(wsl_label(&["Ubuntu".into()]), "WSL2 (Ubuntu)");
        assert_eq!(wsl_label(&["Ubuntu".into(), "Debian".into()]), "WSL2 (Ubuntu, Debian)");
        assert_eq!(wsl_label(&[]), "WSL2");
    }

    /// Test réel : un PowerShell qui écoute et lance un enfant ; kill_tree arrête les deux.
    #[test]
    fn kills_a_real_process_tree() {
        let mut child = std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                "$l=[Net.Sockets.TcpListener]::new([Net.IPAddress]::Loopback,0);$l.Start();powershell -NoProfile -Command Start-Sleep 60",
            ])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("powershell requis pour ce test");
        let pid = child.id();
        let deadline = Instant::now() + Duration::from_secs(25);
        let grandchild = loop {
            let procs = snapshot(&fresh_system());
            let has_port = scan().unwrap().iter().any(|e| e.pid == pid);
            if let (true, Some(g)) = (has_port, procs.iter().find(|p| p.parent == Some(pid))) {
                break g.pid;
            }
            assert!(Instant::now() < deadline, "arbre non établi");
            std::thread::sleep(Duration::from_millis(300));
        };

        let preview = preview_tree(pid, std::process::id()).unwrap();
        assert!(preview.iter().any(|m| m.pid == grandchild && m.role == "child"));
        assert_eq!(kill_tree(pid, std::process::id(), false), Ok(true));
        assert!(child.wait().is_ok());
        std::thread::sleep(Duration::from_millis(500));
        assert!(fresh_system().process(Pid::from_u32(grandchild)).is_none(), "l'enfant doit être arrêté");
    }

    #[test]
    fn scan_finds_own_listener() {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        let me = std::process::id();
        let list = scan().unwrap();
        let e = list
            .iter()
            .find(|e| e.port == port && e.proto == "TCP")
            .expect("le port ouvert par le test doit être listé");
        assert_eq!(e.pid, me);
        assert!(e.addresses.contains(&"127.0.0.1".to_string()));
        assert_eq!(e.source, "Host");
        assert!(e.memory_bytes > 0);
        assert!(!e.protected);
        // Se tuer soi-même est refusé même avec un port valide.
        assert!(kill_listener(me, me).is_err());
    }

    /// Test réel : un PowerShell enfant ouvre un port, kill_listener le termine.
    #[test]
    fn kills_a_real_child_listener() {
        let mut child = std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                "$l=[Net.Sockets.TcpListener]::new([Net.IPAddress]::Loopback,0);$l.Start();Start-Sleep 60",
            ])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("powershell requis pour ce test");
        let pid = child.id();

        let deadline = Instant::now() + Duration::from_secs(20);
        while !scan().unwrap().iter().any(|e| e.pid == pid) {
            assert!(Instant::now() < deadline, "le port de l'enfant n'est pas apparu");
            std::thread::sleep(Duration::from_millis(200));
        }

        let out = kill_listener(pid, std::process::id()).expect("arrêt attendu");
        assert_eq!(out.pid, pid);
        assert!(child.wait().is_ok());
        assert!(!scan().unwrap().iter().any(|e| e.pid == pid));
    }
}
