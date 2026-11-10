// « Copy AI Diagnostic Prompt » : copie un résumé Markdown du port choisi, prêt à coller dans un
// assistant IA (Claude Code, Cursor…). Le texte reste en anglais quelle que soit la langue de l'interface.
import { t } from "./i18n.js";
import { state, $, invoke, inExcluded } from "./state.js";
import { toast } from "./toast.js";

let context = null; // { version, os, elevated } : ne change pas pendant la session

const pad = (n) => String(n).padStart(2, "0");
const stamp = (d) => `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`;
const code = (v) => `\`${String(v).replace(/`/g, "'")}\``;

/** Construit le prompt pour une ligne du tableau. Fonction pure : testable sans l'interface. */
export function buildPrompt(p, { version, os, elevated, shieldName, range, now }) {
  const conflict = shieldName !== undefined && shieldName.toLowerCase() !== p.process_name.toLowerCase();
  const shield = shieldName === undefined
    ? "Off"
    : conflict ? `Active (Conflict: ${code(p.process_name)} vs expected ${code(shieldName)})` : `Active (Monitoring ${code(shieldName)})`;
  const parent = p.parent_pid == null ? "Unknown" : `${code(p.parent_name ?? "?")} (PID: ${code(p.parent_pid)})`;
  const mem = p.memory_bytes > 0 ? code(`${(p.memory_bytes / 1048576).toFixed(1)} MB`) : "Unknown";
  return `### 🩺 PortDoctor Diagnostic Summary

**Context:**
- **Application:** PortDoctor v${version}
- **OS:** ${os}
- **Timestamp:** ${stamp(now)}
- **Elevated Privileges (Admin):** ${elevated ? "Yes" : "No"}

#### 📍 Target Inspection
- **Port:** ${code(p.port)} (${p.proto})
- **Status:** ${conflict ? "SHIELD_ALERT" : "OCCUPIED"}
- **Listening Addresses:** ${p.addresses.map(code).join(", ") || "Unknown"}
- **Process Name:** ${code(p.process_name)}
- **PID:** ${code(p.pid)}
- **Parent Process (PPID):** ${parent}
- **Executable Path:** ${p.exe_path ? code(p.exe_path) : "Unknown"}
- **Memory Footprint:** ${mem}
- **Source:** ${p.source}
- **Protected System Process:** ${p.protected ? "Yes" : "No"}
- **Port Shield State:** ${shield}
- **Hyper-V / Excluded Range:** ${range ? `Yes (inside the ${range.start}-${range.end} range excluded by Windows)` : "No"}

#### ❓ Task for AI Assistant
Analyze the diagnostic data above and provide:
1. The safest command (\`powershell\` or \`cmd\`) to inspect or terminate this process hierarchy without affecting unrelated dev services.
2. The root cause if this appears to be an orphan process, a binding failure (\`0.0.0.0\` vs \`127.0.0.1\`), or a background lock.
3. Recommended steps to release port ${code(p.port)} or reconfigure the application binding.
`;
}

async function copy(text) {
  try { await navigator.clipboard.writeText(text); return; } catch { /* repli ci-dessous */ }
  const ta = document.createElement("textarea");
  ta.value = text;
  ta.style.cssText = "position:fixed;opacity:0";
  document.body.appendChild(ta);
  ta.select();
  const ok = document.execCommand("copy");
  ta.remove();
  if (!ok) throw new Error("clipboard");
}

export function initDiag() {
  $("rows").addEventListener("click", async (e) => {
    const btn = e.target.closest("button.btn-ai-prompt");
    if (!btn) return;
    const p = state.ports.find((x) => `${x.pid}:${x.proto}:${x.port}` === btn.dataset.key);
    if (!p) return;
    try {
      context ??= await invoke("get_diag_context");
      await copy(buildPrompt(p, { ...context, shieldName: state.shield[p.port], range: inExcluded(p.port), now: new Date() }));
      toast(t("diag_copied"), "ai", 3500);
    } catch {
      toast(t("diag_failed"), "err");
    }
  });
}
