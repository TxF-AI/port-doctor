// Thème sombre / clair : mémorisé, appliqué sur <html data-theme>. Le sombre reste le thème par défaut.
import { t } from "./i18n.js";
import { $ } from "./state.js";

function load() {
  try { return localStorage.getItem("theme") === "light" ? "light" : "dark"; } catch { return "dark"; }
}

let theme = load();

/** Met à jour l'attribut du document et le bouton (icône = thème vers lequel on bascule). */
export function applyTheme() {
  document.documentElement.dataset.theme = theme;
  const btn = $("theme");
  btn.textContent = theme === "dark" ? "☀️" : "🌙";
  const label = t(theme === "dark" ? "theme_to_light" : "theme_to_dark");
  btn.title = label;
  btn.setAttribute("aria-label", label);
}

export function initTheme() {
  applyTheme();
  $("theme").addEventListener("click", () => {
    theme = theme === "dark" ? "light" : "dark";
    try { localStorage.setItem("theme", theme); } catch { /* stockage indisponible */ }
    applyTheme();
  });
}
