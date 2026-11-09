// État partagé et utilitaires purs de l'interface.
import { t } from "./i18n.js";

export const invoke = (cmd, args) => window.__TAURI__.core.invoke(cmd, args);


export const state = {
  ports: [],
  ranges: [],
  sort: { key: "port", dir: 1 },
  filter: "",
  devOnly: false,
  armed: null, // clé de la ligne en attente de confirmation du Kill
  armTimer: null,
  backendMem: 0,
  busy: false,
  // Statut du pied de page, gardé sous forme de données pour être retraduit au changement de langue.
  status: { kind: "none" },
};

export const $ = (id) => document.getElementById(id);
export const esc = (s) =>
  String(s ?? "").replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[c]);
export const rowKey = (p) => `${p.pid}:${p.proto}:${p.port}`;
export const fmtMem = (b) =>
  b >= 1073741824 ? `${(b / 1073741824).toFixed(1)} ${t("mem_gb")}` : b >= 1048576 ? `${Math.round(b / 1048576)} ${t("mem_mb")}` : b > 0 ? `${Math.round(b / 1024)} ${t("mem_kb")}` : "—";

// Un port (de dev ou non) tombe-t-il dans une plage exclue par Windows ?
export const inExcluded = (port) => state.ranges.find((r) => port >= r.start && port <= r.end);

export function disarm() { clearTimeout(state.armTimer); state.armed = null; }

// Lignes visibles : filtre dev, recherche texte, tri.
export function visible() {
  const q = state.filter.trim().toLowerCase();
  const { key, dir } = state.sort;
  return state.ports
    .filter((p) => !state.devOnly || p.is_dev_port)
    .filter((p) => !q || `${p.port} ${p.proto} ${p.process_name} ${p.pid} ${p.source} ${p.exe_path ?? ""}`.toLowerCase().includes(q))
    .sort((a, b) => {
      const x = a[key], y = b[key];
      const c = typeof x === "number" ? x - y : String(x).localeCompare(String(y));
      return (c || a.port - b.port || a.pid - b.pid) * dir;
    });
}
