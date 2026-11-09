use std::collections::BTreeMap;
use std::collections::BTreeSet;

use netstat2::{AddressFamilyFlags, ProtocolFlags, ProtocolSocketInfo, TcpState, get_sockets_info};
use sysinfo::{Pid, ProcessesToUpdate, System};

use crate::models::{ExcludedRange, KillOutcome, PortEntry};

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
    .map_err(|e| format!("Lecture des sockets impossible : {e}"))?;

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
                (_, None) => "Inconnu".to_string(),
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
    sys.refresh_processes(ProcessesToUpdate::All, true);
    sys
}

/// Liste les ports TCP en écoute et les sockets UDP liés, avec leur processus.
pub fn scan() -> Result<Vec<PortEntry>, String> {
    let raw = raw_listeners()?;
    let sys = fresh_system();
    Ok(merge(raw, &sys))
}

/// Contrôles avant un arrêt : fonction pure, testée sans toucher à un processus.
pub fn guard_kill(pid: u32, own_pid: u32, entries: &[PortEntry]) -> Result<&PortEntry, String> {
    if pid == own_pid {
        return Err("PortDoctor ne peut pas se terminer lui-même ici.".into());
    }
    // Le PID doit détenir un port MAINTENANT : évite de tuer un processus qui a réutilisé
    // un PID après le dernier scan affiché.
    let entry = entries
        .iter()
        .find(|e| e.pid == pid)
        .ok_or_else(|| format!("Le PID {pid} ne détient plus aucun port : rafraîchis la liste."))?;
    if entry.protected {
        return Err(format!(
            "{} (PID {pid}) est un processus système protégé : arrêt refusé.",
            entry.process_name
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
        .ok_or_else(|| format!("{name} (PID {pid}) s'est déjà arrêté."))?;
    if process.kill() {
        Ok(KillOutcome {
            pid,
            process_name: name,
        })
    } else {
        Err(format!(
            "Accès refusé pour {name} (PID {pid}) : processus lancé en administrateur ? Relance PortDoctor en administrateur."
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
        .map_err(|e| format!("netsh indisponible : {e}"))?;
    if !out.status.success() {
        return Err("netsh a échoué : plages exclues non disponibles.".into());
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
    use std::time::{Duration, Instant};

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
