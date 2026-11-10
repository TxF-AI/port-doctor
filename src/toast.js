// Petites notifications discrètes en bas à droite (succès / erreur d'un arrêt de processus).
import { $, esc } from "./state.js";

export function toast(text, kind = "good") {
  const el = document.createElement("div");
  el.className = `toast ${kind}`;
  el.innerHTML = esc(text);
  $("toasts").appendChild(el);
  setTimeout(() => el.classList.add("out"), 3800);
  setTimeout(() => el.remove(), 4200);
}
