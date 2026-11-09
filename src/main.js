import "./styles.css";
import { t, setLang, getLang, applyStatic } from "./i18n.js";

const invoke = (cmd, args) => window.__TAURI__.core.invoke(cmd, args);

const state = {
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

const $ = (id) => document.getElementById(id);
const esc = (s) =>
  String(s ?? "").replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[c]);
const rowKey = (p) => `${p.pid}:${p.proto}:${p.port}`;
const fmtMem = (b) =>
  b >= 1073741824 ? `${(b / 1073741824).toFixed(1)} ${t("mem_gb")}` : b >= 1048576 ? `${Math.round(b / 1048576)} ${t("mem_mb")}` : b > 0 ? `${Math.round(b / 1024)} ${t("mem_kb")}` : "—";

// Un port (de dev ou non) tombe-t-il dans une plage exclue par Windows ?
const inExcluded = (port) => state.ranges.find((r) => port >= r.start && port <= r.end);

function visible() {
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

function renderStats() {
  const dev = new Set(state.ports.filter((p) => p.is_dev_port).map((p) => p.port)).size;
  const conflicts = state.ports.filter((p) => inExcluded(p.port)).length;
  const active = new Set(state.ports.filter((p) => p.proto === "TCP").map((p) => p.port)).size;
  $("stats").innerHTML = `
    <div class="stat ok">${esc(t("stat_tcp"))}<b>${active}</b></div>
    <div class="stat">${esc(t("stat_dev"))}<b>${dev}</b></div>
    <div class="stat ${conflicts ? "warn" : ""}">${esc(t("stat_conflicts"))}<b>${conflicts}</b></div>
    <div class="stat">${esc(t("stat_ranges"))}<b>${state.ranges.length}</b></div>
    <div class="stat">${esc(t("stat_ram"))}<b>${fmtMem(state.backendMem)}</b></div>`;
}

function renderRows() {
  const list = visible();
  $("empty").hidden = list.length > 0;
  $("rows").innerHTML = list
    .map((p) => {
      const key = rowKey(p);
      const armed = state.armed === key;
      const ex = inExcluded(p.port);
      const tree = p.parent_name && /^(npm|npx|pnpm|yarn|cmd|powershell|pwsh)(\.exe|\.cmd)?$/i.test(p.parent_name);
      const tags =
        (p.is_dev_port ? `<span class="tag info">${esc(t("tag_dev"))}</span>` : "") +
        (ex ? `<span class="tag warn" title="${esc(t("tag_range", { a: ex.start, b: ex.end }))}">${esc(t("tag_hyperv"))}</span>` : "") +
        (tree ? `<span class="tag" title="${esc(t("tag_parent", { name: p.parent_name, pid: p.parent_pid }))}">via ${esc(p.parent_name)}</span>` : "");
      const btn = p.protected
        ? `<button class="kill" disabled title="${esc(t("protected_title"))}">${esc(t("protected"))}</button>`
        : `<button class="kill${armed ? " armed" : ""}" data-key="${esc(key)}" data-pid="${p.pid}">${esc(armed ? t("confirm") : t("kill"))}</button>`;
      return `<tr class="${p.is_dev_port ? "dev" : ""}${ex ? " conflict" : ""}">
        <td class="port">${p.port}</td>
        <td>${esc(p.proto)}</td>
        <td title="${esc(p.exe_path ?? "")}&#10;${esc(p.addresses.join(", "))}">${esc(p.process_name)}${tags}</td>
        <td class="num">${p.pid}</td>
        <td class="num">${fmtMem(p.memory_bytes)}</td>
        <td>${esc(p.source)}</td>
        <td>${btn}</td></tr>`;
    })
    .join("");
  document.querySelectorAll("thead th[data-k]").forEach((th) => th.classList.toggle("sorted", th.dataset.k === state.sort.key));
}

function renderStatus() {
  const s = state.status;
  let html = "…";
  if (s.kind === "ok") html = esc(t("status_ok", { n: s.n, time: s.time.toLocaleTimeString(t("locale")) }));
  else if (s.kind === "good") html = `<span class="good">${esc(s.text(t))}</span>`;
  else if (s.kind === "err") html = `<span class="err">${esc(s.text)}</span>`;
  $("status").innerHTML = html;
}

function renderBanner() {
  const b = $("banner");
  const hit = state.ports.filter((p) => p.is_dev_port && inExcluded(p.port));
  b.hidden = hit.length === 0;
  b.className = "banner warn";
  if (hit.length) b.textContent = t("banner", { ports: [...new Set(hit.map((p) => p.port))].join(", ") });
}

function renderAll() {
  renderStats(); renderBanner(); renderRows(); renderStatus();
}

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

function disarm() { clearTimeout(state.armTimer); state.armed = null; }

// ── Smart Kill : modale listant l'arbre parent/enfant avant d'arrêter ──
const dlg = $("treeDlg");
let treePlan = null; // { pid, members }

function renderTree() {
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

async function openTree(pid, members) {
  treePlan = { pid, members };
  $("parentsRow").hidden = !members.some((m) => m.role === "parent");
  $("withParents").checked = true;
  renderTree();
  dlg.showModal();
}

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

async function onKill(btn) {
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

$("rows").addEventListener("click", (e) => {
  const btn = e.target.closest("button.kill[data-key]");
  if (btn) onKill(btn);
});
document.querySelector("thead").addEventListener("click", (e) => {
  const th = e.target.closest("th[data-k]");
  if (!th) return;
  state.sort = { key: th.dataset.k, dir: state.sort.key === th.dataset.k ? -state.sort.dir : 1 };
  renderRows();
});
$("filter").addEventListener("input", (e) => { state.filter = e.target.value; renderRows(); });
$("devOnly").addEventListener("change", (e) => { state.devOnly = e.target.checked; renderRows(); });
$("refresh").addEventListener("click", () => refresh(true));

// ── Langue : sélecteur FR / EN, transmis au backend (messages d'erreur et menu du tray) ──
function applyLanguage() {
  applyStatic();
  $("lang").value = getLang();
  invoke("set_language", { lang: getLang() }).catch(() => {});
  if (dlg.open && treePlan) renderTree();
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
