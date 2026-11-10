// Traductions de l'interface (FR / EN). Les entrées peuvent être des fonctions pour les pluriels.
const plural = (n, one, many) => (n > 1 ? many : one);

const DICT = {
  fr: {
    filter_ph: "Filtrer port / processus…  (Ctrl+F)",
    dev_only: "Dev ports",
    refresh: "Rafraîchir",
    refresh_title: "Rafraîchir (F5)",
    language: "Langue",
    col_port: "Port",
    col_proto: "Proto",
    col_process: "Processus",
    col_pid: "PID",
    col_memory: "Mémoire",
    col_source: "Source",
    col_action: "Action",
    stat_tcp: "Ports TCP actifs",
    stat_dev: "Dev ports",
    stat_conflicts: "Conflits Hyper-V",
    stat_ranges: "Plages exclues",
    stat_ram: "RAM backend",
    empty: "Aucun port ne correspond.",
    banner: ({ ports }) => `Windows réserve des plages de ports : ${ports} tombe(nt) dans une plage exclue (Hyper-V / WinNAT).`,
    status_ok: ({ n, time }) => `${n} entrées · mis à jour ${time}`,
    kill: "Kill",
    confirm: "Confirmer ?",
    protected: "Protégé",
    protected_title: "Processus système protégé",
    tag_dev: "dev",
    tag_hyperv: "Hyper-V",
    tag_range: ({ a, b }) => `Plage exclue ${a}-${b}`,
    tag_parent: ({ name, pid }) => `Processus parent : ${name} (PID ${pid})`,
    killed: ({ name, pid }) => `${name} (PID ${pid}) arrêté.`,
    tree_killed: ({ pid }) => `Arbre du PID ${pid} arrêté.`,
    dlg_title: "Arrêter cet arbre de processus ?",
    dlg_intro: ({ n, p, k }) =>
      `${n} processus seront arrêtés de force${p ? ` (dont ${p} ${plural(p, "parent", "parents")})` : ""}${k ? `, ${k} ${plural(k, "enfant", "enfants")}` : ""}.`,
    role_parent: "Parent",
    role_target: "Port",
    role_child: "Enfant",
    dlg_parents: "Inclure les processus parents (lanceurs npm / yarn…)",
    dlg_note: "Les processus système (svchost, lsass, System…) ne sont jamais arrêtés : s'il y en a un dans l'arbre, l'opération est refusée.",
    cancel: "Annuler",
    stop_tree: "Arrêter l'arbre",
    simple_view: "Vue simple",
    simple_title: "Masque les PID et les détails techniques",
    col_what: "Ce qui l'utilise",
    free_port: "Libérer",
    toast_freed: ({ port, name }) => `Port ${port} libéré (${name} arrêté).`,
    toast_tree_freed: ({ port }) => `Port ${port} libéré (arbre de processus arrêté).`,
    diag_title: "Copier un diagnostic de ce port pour un assistant IA (Claude Code, Cursor…)",
    diag_copied: "Diagnostic IA copié dans le presse-papiers. Prêt à coller dans votre assistant.",
    diag_failed: "Impossible de copier le diagnostic dans le presse-papiers.",
    shield_off_title: "Surveiller ce port (Port Shield) : alerte si un autre processus le prend",
    shield_on_title: "Port Shield actif sur ce port (cliquer pour retirer)",
    shield_added: ({ port, name }) => `Port Shield : le port ${port} est surveillé (attendu : ${name}).`,
    shield_removed: ({ port }) => `Port Shield : le port ${port} n'est plus surveillé.`,
    shield_taken: ({ port, name, pid, expected }) => `Port Shield : le port ${port} est maintenant utilisé par ${name} (PID ${pid}) au lieu de ${expected}.`,
    shield_free: ({ port, expected }) => `Port Shield : le port ${port} a été libéré (${expected} ne l'occupe plus).`,
    kind_frontend: "Serveur web de développement (Vite, React…)",
    kind_node: "Application Node.js",
    kind_python: "Application Python",
    kind_java: "Application Java",
    kind_dotnet: "Application .NET",
    kind_database: "Base de données",
    kind_docker: "Conteneur Docker",
    kind_wsl: "Programme lancé dans WSL2",
    kind_system: "Processus système",
    kind_other: "Autre programme",
    mem_gb: "Go",
    mem_mb: "Mo",
    mem_kb: "Ko",
    locale: "fr-FR",
  },
  en: {
    filter_ph: "Filter port / process…  (Ctrl+F)",
    dev_only: "Dev ports",
    refresh: "Refresh",
    refresh_title: "Refresh (F5)",
    language: "Language",
    col_port: "Port",
    col_proto: "Proto",
    col_process: "Process",
    col_pid: "PID",
    col_memory: "Memory",
    col_source: "Source",
    col_action: "Action",
    stat_tcp: "Active TCP ports",
    stat_dev: "Dev ports",
    stat_conflicts: "Hyper-V conflicts",
    stat_ranges: "Excluded ranges",
    stat_ram: "Backend RAM",
    empty: "No matching port.",
    banner: ({ ports }) => `Port(s) ${ports} fall inside a range reserved by Windows (Hyper-V / WinNAT).`,
    status_ok: ({ n, time }) => `${n} entries · updated ${time}`,
    kill: "Kill",
    confirm: "Confirm?",
    protected: "Protected",
    protected_title: "Protected system process",
    tag_dev: "dev",
    tag_hyperv: "Hyper-V",
    tag_range: ({ a, b }) => `Excluded range ${a}-${b}`,
    tag_parent: ({ name, pid }) => `Parent process: ${name} (PID ${pid})`,
    killed: ({ name, pid }) => `${name} (PID ${pid}) stopped.`,
    tree_killed: ({ pid }) => `Process tree of PID ${pid} stopped.`,
    dlg_title: "Stop this process tree?",
    dlg_intro: ({ n, p, k }) =>
      `${n} ${plural(n, "process", "processes")} will be force-stopped${p ? ` (including ${p} ${plural(p, "parent", "parents")})` : ""}${k ? `, ${k} ${plural(k, "child", "children")}` : ""}.`,
    role_parent: "Parent",
    role_target: "Port",
    role_child: "Child",
    dlg_parents: "Include parent processes (npm / yarn launchers…)",
    dlg_note: "System processes (svchost, lsass, System…) are never stopped: if one is in the tree, the operation is refused.",
    cancel: "Cancel",
    stop_tree: "Stop the tree",
    simple_view: "Simple view",
    simple_title: "Hides PIDs and technical details",
    col_what: "What's using it",
    free_port: "Free port",
    toast_freed: ({ port, name }) => `Port ${port} freed (${name} stopped).`,
    toast_tree_freed: ({ port }) => `Port ${port} freed (process tree stopped).`,
    diag_title: "Copy a diagnostic of this port for an AI assistant (Claude Code, Cursor…)",
    diag_copied: "AI diagnostic prompt copied to clipboard. Ready to paste into your assistant.",
    diag_failed: "Could not copy the diagnostic to the clipboard.",
    shield_off_title: "Watch this port (Port Shield): alerts you if another process takes it",
    shield_on_title: "Port Shield is on for this port (click to remove)",
    shield_added: ({ port, name }) => `Port Shield: port ${port} is watched (expected: ${name}).`,
    shield_removed: ({ port }) => `Port Shield: port ${port} is no longer watched.`,
    shield_taken: ({ port, name, pid, expected }) => `Port Shield: port ${port} is now held by ${name} (PID ${pid}) instead of ${expected}.`,
    shield_free: ({ port, expected }) => `Port Shield: port ${port} was released (${expected} no longer holds it).`,
    kind_frontend: "Dev web server (Vite, React…)",
    kind_node: "Node.js app",
    kind_python: "Python app",
    kind_java: "Java app",
    kind_dotnet: ".NET app",
    kind_database: "Database",
    kind_docker: "Docker container",
    kind_wsl: "Program running in WSL2",
    kind_system: "System process",
    kind_other: "Other program",
    mem_gb: "GB",
    mem_mb: "MB",
    mem_kb: "KB",
    locale: "en-US",
  },
};

function detect() {
  try {
    const saved = localStorage.getItem("lang");
    if (saved === "fr" || saved === "en") return saved;
  } catch { /* stockage indisponible */ }
  return (navigator.language || "en").toLowerCase().startsWith("fr") ? "fr" : "en";
}

let lang = detect();

export const getLang = () => lang;

export function t(key, params = {}) {
  const v = DICT[lang][key] ?? DICT.fr[key] ?? key;
  return typeof v === "function" ? v(params) : v;
}

/** Applique les textes statiques marqués data-i18n / data-i18n-ph / data-i18n-title. */
export function applyStatic() {
  document.documentElement.lang = lang;
  document.querySelectorAll("[data-i18n]").forEach((e) => (e.textContent = t(e.dataset.i18n)));
  document.querySelectorAll("[data-i18n-ph]").forEach((e) => (e.placeholder = t(e.dataset.i18nPh)));
  document.querySelectorAll("[data-i18n-title]").forEach((e) => (e.title = t(e.dataset.i18nTitle)));
}

export function setLang(next) {
  if (next !== "fr" && next !== "en") return;
  lang = next;
  try { localStorage.setItem("lang", next); } catch { /* ignoré */ }
  applyStatic();
}
