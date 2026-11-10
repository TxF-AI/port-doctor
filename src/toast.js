// Petites notifications discrètes en bas à droite (succès / erreur d'un arrêt de processus, alertes du Shield).
import { $, esc } from "./state.js";

export function toast(text, kind = "good", ms = 4200) {
  const el = document.createElement("div");
  el.className = `toast ${kind}`;
  el.innerHTML = esc(text);
  $("toasts").appendChild(el);
  setTimeout(() => el.classList.add("out"), ms - 400);
  setTimeout(() => el.remove(), ms);
}
