// Bouton Kill : confirmation en deux temps, ou Smart Kill si le processus a un arbre.
import { state, $, invoke, disarm } from "./state.js";
import { t } from "./i18n.js";
import { renderRows, renderStatus, fadeOutRow } from "./render.js";
import { openTree } from "./tree-dialog.js";
import { toast } from "./toast.js";
import { unshield } from "./shield.js";

async function onKill(btn, refresh) {
  const key = btn.dataset.key;
  if (state.armed !== key) {
    // Un arbre (enfants ou lanceur npm/yarn) ? → modale explicite. Sinon : bouton armé 4 s.
    try {
      const members = await invoke("preview_kill_tree", { pid: Number(btn.dataset.pid) });
      if (members.length > 1) {
        disarm();
        return openTree(Number(btn.dataset.pid), members, btn.dataset.port);
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
    unshield(btn.dataset.port);
    toast(t("toast_freed", { port: btn.dataset.port, name: r.process_name }));
    await fadeOutRow(pid);
  } catch (e) {
    status = { kind: "err", text: String(e) };
    toast(String(e), "err");
  }
  await refresh();
  state.status = status;
  renderStatus();
}

export function initKill(refresh) {
  for (const id of ["rows", "quick"]) {
    $(id).addEventListener("click", (e) => {
      const btn = e.target.closest("button.kill[data-key]");
      if (btn) onKill(btn, refresh);
    });
  }
}
