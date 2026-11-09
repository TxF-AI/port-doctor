// Bouton Kill : confirmation en deux temps, ou Smart Kill si le processus a un arbre.
import { state, $, invoke, disarm } from "./state.js";
import { renderRows, renderStatus } from "./render.js";
import { openTree } from "./tree-dialog.js";

async function onKill(btn, refresh) {
  const key = btn.dataset.key;
  if (state.armed !== key) {
    // Un arbre (enfants ou lanceur npm/yarn) ? → modale explicite. Sinon : bouton armé 4 s.
    try {
      const members = await invoke("preview_kill_tree", { pid: Number(btn.dataset.pid) });
      if (members.length > 1) {
        disarm();
        return openTree(Number(btn.dataset.pid), members);
      }
    } catch (e) {
      state.status = { kind: "err", text: String(e) };
      return renderStatus();
    }
    disarm();
    state.armed = key;
    state.armTimer = setTimeout(() => { state.armed = null; renderRows(); }, 4000);
    renderRows();
    return;
  }
  disarm();
  const pid = Number(btn.dataset.pid);
  let status;
  try {
    const r = await invoke("kill_port_process", { pid });
    status = { kind: "good", text: (tr) => tr("killed", { name: r.process_name, pid: r.pid }) };
  } catch (e) {
    status = { kind: "err", text: String(e) };
  }
  await refresh();
  state.status = status;
  renderStatus();
}

export function initKill(refresh) {
  $("rows").addEventListener("click", (e) => {
    const btn = e.target.closest("button.kill[data-key]");
    if (btn) onKill(btn, refresh);
  });
}
