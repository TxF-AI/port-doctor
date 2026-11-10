// Rendu du tableau, des compteurs, de la bannière et du pied de page.
import { t } from "./i18n.js";
import { state, $, esc, rowKey, fmtMem, inExcluded, visible } from "./state.js";

export function renderStats() {
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

export function renderRows() {
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
        : `<button class="kill${armed ? " armed" : ""}" data-key="${esc(key)}" data-pid="${p.pid}" data-port="${p.port}">${esc(armed ? t("confirm") : t(state.simple ? "free_port" : "kill"))}</button>`;
      const on = state.shield[p.port] !== undefined;
      // Sans bouclier (processus protégé ou UDP) : un emplacement vide de même largeur garde les boutons alignés.
      const shield = p.protected || p.proto !== "TCP"
        ? `<button class="shield" style="visibility:hidden" disabled tabindex="-1" aria-hidden="true">🛡</button>`
        : `<button class="shield${on ? " on" : ""}" data-port="${p.port}" data-name="${esc(p.process_name)}" aria-pressed="${on}" title="${esc(t(on ? "shield_on_title" : "shield_off_title"))}">🛡</button>`;
      const diag = `<button class="btn-ai-prompt" data-key="${esc(key)}" title="${esc(t("diag_title"))}">📋 AI Prompt</button>`;
      const alert = on && state.shield[p.port].toLowerCase() !== p.process_name.toLowerCase();
      const badge = on ? `<span class="status-badge ${alert ? "alert" : "shielded"}">${esc(t(alert ? "badge_alert" : "badge_shielded"))}</span>` : "";
      // Vue Simple : un libellé clair à la place du nom de l'exécutable (celui-ci reste dans l'infobulle).
      const what = state.simple ? esc(t(`kind_${p.kind}`)) : esc(p.process_name);
      const hover = `${p.process_name}${state.simple ? ` (PID ${p.pid})` : ""}\n${p.exe_path ?? ""}\n${p.addresses.join(", ")}`;
      return `<tr data-pid="${p.pid}" class="${p.is_dev_port ? "dev" : ""}${ex ? " conflict" : ""}">
        <td class="port">${p.port}</td>
        <td class="adv">${esc(p.proto)}</td>
        <td title="${esc(hover)}">${what}${tags}${badge}</td>
        <td class="num adv">${p.pid}</td>
        <td class="num adv">${fmtMem(p.memory_bytes)}</td>
        <td class="adv">${esc(p.source)}</td>
        <td>${shield}${diag}${btn}</td></tr>`;
    })
    .join("");
  document.querySelectorAll("thead th[data-k]").forEach((th) => th.classList.toggle("sorted", th.dataset.k === state.sort.key));
}

/** Fait disparaître en fondu la ligne d'un processus qu'on vient d'arrêter, avant le rafraîchissement. */
export function fadeOutRow(pid) {
  const rows = $("rows").querySelectorAll(`tr[data-pid="${pid}"]`);
  if (!rows.length) return Promise.resolve();
  rows.forEach((r) => r.classList.add("leaving"));
  return new Promise((done) => setTimeout(done, 280));
}

export function renderStatus() {
  const s = state.status;
  let html = "…";
  if (s.kind === "ok") html = esc(t("status_ok", { n: s.n, time: s.time.toLocaleTimeString(t("locale")) }));
  else if (s.kind === "good") html = `<span class="good">${esc(s.text(t))}</span>`;
  else if (s.kind === "err") html = `<span class="err">${esc(s.text)}</span>`;
  $("status").innerHTML = html;
}

export function renderBanner() {
  const b = $("banner");
  const hit = state.ports.filter((p) => p.is_dev_port && inExcluded(p.port));
  b.hidden = hit.length === 0;
  b.className = "banner warn";
  if (hit.length) b.textContent = t("banner", { ports: [...new Set(hit.map((p) => p.port))].join(", ") });
}

export function renderAll() {
  renderStats(); renderBanner(); renderRows(); renderStatus();
}
