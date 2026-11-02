// Actualizaciones de Klippo desde GitHub Releases (latest.json firmado con la clave del
// actualizador de Tauri). Como en Android: como mucho una comprobación al día al abrir,
// desactivable en Ajustes, y a mano desde Ajustes → Actualizaciones.
import { check, type Update } from "@tauri-apps/plugin-updater";
import { guardar, leer } from "./ajustes";

const UN_DIA = 24 * 60 * 60 * 1000;

export const actualizacion = $state({
  /** Versión nueva encontrada (null si no hay o aún no se ha mirado) */
  disponible: null as Update | null,
  /** Porcentaje de la descarga mientras se actualiza */
  progreso: null as number | null,
  error: null as string | null,
});

export async function comprobarAlAbrir(): Promise<void> {
  // En desarrollo no: la versión compilada en local no es la publicada
  if (import.meta.env.DEV) return;
  if (!(await leer("buscarActualizaciones", true))) return;
  if (Date.now() - (await leer("ultimaComprobacion", 0)) < UN_DIA) return;
  await guardar("ultimaComprobacion", Date.now());
  try {
    actualizacion.disponible = await check();
  } catch {
    // Sin conexión o GitHub caído: se volverá a mirar mañana
  }
}

/** Comprobación a mano. Devuelve true si hay versión nueva; lanza error si no se pudo mirar */
export async function buscarAhora(): Promise<boolean> {
  await guardar("ultimaComprobacion", Date.now());
  actualizacion.disponible = await check();
  return actualizacion.disponible !== null;
}

export async function instalar(): Promise<void> {
  const nueva = actualizacion.disponible;
  if (!nueva || actualizacion.progreso !== null) return;
  let total = 0;
  let recibido = 0;
  actualizacion.error = null;
  actualizacion.progreso = 0;
  try {
    // En Windows el instalador cierra Klippo, se instala sin preguntar y lo vuelve a abrir
    await nueva.downloadAndInstall((evento) => {
      if (evento.event === "Started") total = evento.data.contentLength ?? 0;
      if (evento.event === "Progress") {
        recibido += evento.data.chunkLength;
        actualizacion.progreso = total ? Math.min(100, Math.round((recibido * 100) / total)) : 0;
      }
    });
  } catch (e) {
    actualizacion.error = String(e);
    actualizacion.progreso = null;
  }
}
