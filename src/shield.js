// Port Shield : surveille des ports choisis et prévient (jamais d'arrêt automatique) quand un autre
// processus les prend ou qu'ils sont libérés. La surveillance tourne côté Rust ; ici on garde la liste.
import { t } from "./i18n.js";
import { state, $, invoke } from "./state.js";
import { renderRows } from "./render.js";
import { toast } from "./toast.js";

function save() {
  try { localStorage.setItem("shield", JSON.stringify(state.shield)); } catch { /* stockage indisponible */ }
  const entries = Object.entries(state.shield).map(([port, name]) => ({ port: Number(port), name }));
  return invoke("set_shield", { entries }).catch(() => {});
}

/** Retire un port de la surveillance (utilisé après un Kill voulu : le port libéré n'est pas une alerte). */
export function unshield(port) {
  if (state.shield[port] === undefined) return;
  delete state.shield[port];
  save();
}

function toggle(port, name) {
  if (state.shield[port] !== undefined) {
    delete state.shield[port];
    toast(t("shield_removed", { port }));
  } else {
    state.shield[port] = name;
    toast(t("shield_added", { port, name }));
  }
  save();
  renderRows();
}

export function initShield() {
  save(); // renvoie au backend la liste mémorisée
  $("rows").addEventListener("click", (e) => {
    const btn = e.target.closest("button.shield");
    if (btn) toggle(btn.dataset.port, btn.dataset.name);
  });
  window.__TAURI__.event.listen("shield-alert", ({ payload: a }) => {
    const text = a.kind === "taken"
      ? t("shield_taken", { port: a.port, name: a.process_name, pid: a.pid, expected: a.expected })
      : t("shield_free", { port: a.port, expected: a.expected });
    toast(text, "warn", 12000);
  });
}
