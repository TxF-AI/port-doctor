use serde::Serialize;

/// Un port en écoute et le processus qui le détient.
/// Les adresses IPv4/IPv6 d'un même (protocole, port, PID) sont fusionnées sur une ligne.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PortEntry {
    pub port: u16,
    /// "TCP" ou "UDP".
    pub proto: String,
    /// Adresses d'écoute (`0.0.0.0`, `127.0.0.1`, `::1`…), triées, sans doublon.
    pub addresses: Vec<String>,
    pub pid: u32,
    pub process_name: String,
    pub exe_path: Option<String>,
    pub memory_bytes: u64,
    /// "Host", "WSL2" ou "Docker" (déduit du nom du processus qui porte le port).
    pub source: String,
    pub is_dev_port: bool,
    /// Processus système : le bouton Kill est refusé par le backend.
    pub protected: bool,
    pub parent_pid: Option<u32>,
    pub parent_name: Option<String>,
}

/// Plage de ports TCP exclue par Windows (Hyper-V, WinNAT, réservations admin).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ExcludedRange {
    pub start: u16,
    pub end: u16,
    /// `*` dans la sortie de netsh : exclusion posée par un administrateur (permanente).
    pub administered: bool,
}

/// Résultat d'un arrêt de processus.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct KillOutcome {
    pub pid: u32,
    pub process_name: String,
}
