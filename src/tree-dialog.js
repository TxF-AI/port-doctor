// Smart Kill : modale listant l'arbre parent/enfant avant d'arrêter.
import { t } from "./i18n.js";
import { state, $, esc, invoke } from "./state.js";
import { renderStatus } from "./render.js";

const dlg = $("treeDlg");
let treePlan = null; // { pid, members }

export const isTreeOpen = () => dlg.open && treePlan !== null;

export function renderTree() {
  const withParents = $("withParents").checked;
  const rows = treePlan.members.filter((m) => withParents || m.role !== "parent");
  const k = rows.filter((m) => m.role === "child").length;
  const p = rows.filter((m) => m.role === "parent").length;
  $("treeIntro").textContent = t("dlg_intro", { n: rows.length, p, k });
  const label = { parent: t("role_parent"), target: t("role_target"), child: t("role_child") };
  $("treeList").innerHTML = rows
    .map((m) => `<li class="${m.role}"><span class="role">${esc(label[m.role])}</span><b>${esc(m.name)}</b><span class="dim">PID ${m.pid}</span></li>`)
    .join("");
}

export function openTree(pid, members) {
  treePlan = { pid, members };
  $("parentsRow").hidden = !members.some((m) => m.role === "parent");
  $("withParents").checked = true;
  renderTree();
  dlg.showModal();
}

// `refresh` est injecté par main.js (évite une dépendance circulaire).
export function initTreeDialog(refresh) {
  $("withParents").addEventListener("change", renderTree);
  $("treeCancel").addEventListener("click", () => dlg.close());
  $("treeOk").addEventListener("click", async () => {
    const { pid } = treePlan;
    const withParents = $("withParents").checked && !$("parentsRow").hidden;
    dlg.close();
    let status;
    try {
      await invoke("kill_process_tree", { pid, includeParents: withParents });
      status = { kind: "good", text: (tr) => tr("tree_killed", { pid }) };
    } catch (e) {
      status = { kind: "err", text: String(e) };
    }
    await refresh();
    state.status = status; // refresh() remplace le statut : on réaffiche le résultat du Kill
    renderStatus();
  });
}
