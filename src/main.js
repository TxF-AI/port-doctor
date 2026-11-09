import "./styles.css";

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
};

const $ = (id) => document.getElementById(id);
const esc = (s) =>
  String(s ?? "").replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[c]);
const rowKey = (p) => `${p.pid}:${p.proto}:${p.port}`;
const fmtMem = (b) => (b >= 1073741824 ? `${(b / 1073741824).toFixed(1)} Go` : b >= 1048576 ? `${Math.round(b / 1048576)} Mo` : b > 0 ? `${Math.round(b / 1024)} Ko` : "—");

// Un port de dev (ou n'importe quel port) tombe-t-il dans une plage exclue par Windows ?
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
    <div class="stat ok">Ports TCP actifs<b>${active}</b></div>
    <div class="stat">Dev ports<b>${dev}</b></div>
    <div class="stat ${conflicts ? "warn" : ""}">Conflits Hyper-V<b>${conflicts}</b></div>
    <div class="stat">Plages exclues<b>${state.ranges.length}</b></div>
    <div class="stat">RAM backend<b>${fmtMem(state.backendMem)}</b></div>`;
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
        (p.is_dev_port ? '<span class="tag info">dev</span>' : "") +
        (ex ? `<span class="tag warn" title="Plage exclue ${ex.start}-${ex.end}">Hyper-V</span>` : "") +
        (tree ? `<span class="tag" title="Processus parent : ${esc(p.parent_name)} (PID ${p.parent_pid})">via ${esc(p.parent_name)}</span>` : "");
      const btn = p.protected
        ? `<button class="kill" disabled title="Processus système protégé">Protégé</button>`
        : `<button class="kill${armed ? " armed" : ""}" data-key="${esc(key)}" data-pid="${p.pid}">${armed ? "Confirmer ?" : "Kill"}</button>`;
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

function setStatus(html) { $("status").innerHTML = html; }

function renderBanner() {
  const b = $("banner");
  const hit = state.ports.filter((p) => p.is_dev_port && inExcluded(p.port));
  b.hidden = hit.length === 0;
  b.className = "banner warn";
  if (hit.length) b.textContent = `Windows réserve des plages de ports : ${[...new Set(hit.map((p) => p.port))].join(", ")} tombe(nt) dans une plage exclue (Hyper-V / WinNAT).`;
}

async function refresh(manual = false) {
  if (state.busy) return;
  state.busy = true;
  try {
    const [ports, mem] = await Promise.all([invoke("get_active_ports"), invoke("get_backend_memory")]);
    state.ports = ports;
    state.backendMem = mem;
    $("led").className = "led ok";
    setStatus(`${ports.length} entrées · mis à jour ${new Date().toLocaleTimeString("fr-FR")}`);
  } catch (e) {
    $("led").className = "led bad";
    setStatus(`<span class="err">${esc(e)}</span>`);
  } finally {
    state.busy = false;
  }
  if (manual || !state.ranges.length) loadRanges();
  renderStats(); renderBanner(); renderRows();
}

async function loadRanges() {
  try { state.ranges = await invoke("get_excluded_ranges"); } catch { state.ranges = []; }
  renderStats(); renderBanner(); renderRows();
}

function disarm() { clearTimeout(state.armTimer); state.armed = null; }

async function onKill(btn) {
  const key = btn.dataset.key;
  if (state.armed !== key) {
    // 1er clic : armer le bouton 4 s (confirmation visuelle).
    disarm();
    state.armed = key;
    state.armTimer = setTimeout(() => { state.armed = null; renderRows(); }, 4000);
    renderRows();
    return;
  }
  disarm();
  const pid = Number(btn.dataset.pid);
  let msg;
  try {
    const r = await invoke("kill_port_process", { pid });
    msg = `<span class="good">${esc(r.process_name)} (PID ${r.pid}) arrêté.</span>`;
  } catch (e) {
    msg = `<span class="err">${esc(e)}</span>`;
  }
  await refresh();
  setStatus(msg); // refresh() remplace le statut : on réaffiche le résultat du Kill
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
window.addEventListener("keydown", (e) => {
  if (e.ctrlKey && e.key.toLowerCase() === "f") { e.preventDefault(); $("filter").focus(); }
  else if (e.key === "F5") { e.preventDefault(); refresh(true); }
  else if (e.key === "Escape") { disarm(); renderRows(); }
});

refresh(true);
// Rafraîchissement automatique seulement si la fenêtre est visible (tray : pas de travail inutile).
setInterval(() => { if (!document.hidden) refresh(); }, 3000);
