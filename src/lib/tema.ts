// Tema claro, oscuro o el de Windows (como en Android: Ajustes → Apariencia → Tema)
import { getCurrentWindow } from "@tauri-apps/api/window";
import { leer } from "./ajustes";

export type Tema = "sistema" | "claro" | "oscuro";

const oscuroDelSistema = typeof window !== "undefined" ? window.matchMedia("(prefers-color-scheme: dark)") : null;
let preferido: Tema = "sistema";

function pintar() {
  const oscuro = preferido === "oscuro" || (preferido === "sistema" && !!oscuroDelSistema?.matches);
  document.documentElement.dataset.tema = oscuro ? "oscuro" : "claro";
}

/** Aplica el tema a la página y a la barra de título de la ventana */
export function aplicarTema(tema: Tema): void {
  preferido = tema;
  pintar();
  getCurrentWindow()
    .setTheme(tema === "sistema" ? null : tema === "oscuro" ? "dark" : "light")
    .catch(() => {});
}

/** Al arrancar: el tema guardado, y seguir los cambios de Windows si es "sistema" */
export async function iniciarTema(): Promise<void> {
  oscuroDelSistema?.addEventListener("change", pintar);
  aplicarTema(await leer<Tema>("tema", "sistema"));
}
