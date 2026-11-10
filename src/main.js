// Point d'entrée : rafraîchissement, événements globaux, langue.
import "./styles.css";
import { setLang, getLang, applyStatic } from "./i18n.js";
import { state, $, invoke, disarm, setSimple } from "./state.js";
import { renderAll, renderRows } from "./render.js";
import { initKill } from "./kill.js";
import { initShield } from "./shield.js";
import { initTreeDialog, isTreeOpen, renderTree } from "./tree-dialog.js";

async function refresh(manual = false) {
  if (state.busy) return;
  state.busy = true;
  try {
    const [ports, mem] = await Promise.all([invoke("get_active_ports"), invoke("get_backend_memory")]);
    state.ports = ports;
    state.backendMem = mem;
    $("led").className = "led ok";
    state.status = { kind: "ok", n: ports.length, time: new Date() };
  } catch (e) {
    $("led").className = "led bad";
    state.status = { kind: "err", text: String(e) };
  } finally {
    state.busy = false;
  }
  if (manual || !state.ranges.length) loadRanges();
  renderAll();
}

async function loadRanges() {
  try { state.ranges = await invoke("get_excluded_ranges"); } catch { state.ranges = []; }
  renderAll();
}

initTreeDialog(refresh);
initKill(refresh);
initShield();

document.querySelector("thead").addEventListener("click", (e) => {
  const th = e.target.closest("th[data-k]");
  if (!th) return;
  state.sort = { key: th.dataset.k, dir: state.sort.key === th.dataset.k ? -state.sort.dir : 1 };
  renderRows();
});
$("filter").addEventListener("input", (e) => { state.filter = e.target.value; renderRows(); });
$("devOnly").addEventListener("change", (e) => { state.devOnly = e.target.checked; renderRows(); });
$("refresh").addEventListener("click", () => refresh(true));

// Vue Simple / Avancée : mémorisée, appliquée avant le premier rendu.
$("simple").checked = state.simple;
setSimple(state.simple);
$("simple").addEventListener("change", (e) => { disarm(); setSimple(e.target.checked); renderRows(); });

// ── Langue : sélecteur FR / EN, transmis au backend (messages d'erreur et menu du tray) ──
function applyLanguage() {
  applyStatic();
  $("lang").value = getLang();
  invoke("set_language", { lang: getLang() }).catch(() => {});
  if (isTreeOpen()) renderTree();
  renderAll();
}
$("lang").addEventListener("change", (e) => {
  setLang(e.target.value);
  applyLanguage();
});

window.addEventListener("keydown", (e) => {
  if (e.ctrlKey && e.key.toLowerCase() === "f") { e.preventDefault(); $("filter").focus(); }
  else if (e.key === "F5") { e.preventDefault(); refresh(true); }
  else if (e.key === "Escape") { disarm(); renderRows(); }
});

applyLanguage();
refresh(true);
// Rafraîchissement automatique seulement si la fenêtre est visible (tray : pas de travail inutile).
setInterval(() => { if (!document.hidden) refresh(); }, 3000);
