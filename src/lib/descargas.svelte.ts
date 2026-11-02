// Lista de descargas: la lleva el lado de Rust y avisa de cada cambio con el evento "descargas"
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export type Calidad = "mejor" | "p1080" | "p720" | "p480" | "audio";
export const CALIDADES: Calidad[] = ["mejor", "p1080", "p720", "p480", "audio"];

export type Estado =
  | { tipo: "analizando" }
  | { tipo: "descargando" }
  | { tipo: "procesando" }
  | { tipo: "completada" }
  | { tipo: "cancelada" }
  | { tipo: "fallida"; mensaje: string };

export interface Descarga {
  id: string;
  url: string;
  calidad: Calidad;
  creado: number;
  titulo: string | null;
  estado: Estado;
  /** De 0 a 1; null mientras no se sabe */
  progreso: number | null;
  /** Segundos que faltan */
  eta: number | null;
  archivo: string | null;
  miniatura: string | null;
  /** Segundos */
  duracion: number | null;
  esAudio: boolean;
}

export const descargas = $state({ lista: [] as Descarga[] });

let escuchando = false;

export async function empezarAEscuchar(): Promise<void> {
  if (escuchando) return;
  escuchando = true;
  await listen<Descarga[]>("descargas", (evento) => {
    descargas.lista = evento.payload;
  });
  descargas.lista = await invoke<Descarga[]>("listar_descargas");
}

export function estaActiva(d: Descarga): boolean {
  return d.estado.tipo === "analizando" || d.estado.tipo === "descargando" || d.estado.tipo === "procesando";
}

export const descargar = (url: string, calidad: Calidad) => invoke<string>("descargar", { url, calidad });
export const cancelar = (id: string) => invoke("cancelar_descarga", { id });
export const reintentar = (id: string) => invoke("reintentar_descarga", { id });
export const eliminar = (id: string) => invoke("eliminar_descarga", { id });
export const reproducir = (id: string) => invoke("reproducir", { id });
export const mostrarEnCarpeta = (id: string) => invoke("mostrar_en_carpeta", { id });

/** 75 → "1:15"; 3725 → "1:02:05" */
export function formatearDuracion(segundos: number): string {
  const s = Math.round(segundos);
  const horas = Math.floor(s / 3600);
  const minutos = Math.floor((s % 3600) / 60);
  const resto = String(s % 60).padStart(2, "0");
  return horas > 0 ? `${horas}:${String(minutos).padStart(2, "0")}:${resto}` : `${minutos}:${resto}`;
}

/** "[Instagram] ABC123: Instagram sent an empty media response. Check if..." →
 *  "Instagram sent an empty media response." */
export function resumirError(mensaje: string): string {
  const sinPrefijo = mensaje.replace(/^\[[^\]]+]\s*[^:\s]+:\s*/, "");
  const fin = sinPrefijo.indexOf(". ");
  return fin > 0 ? sinPrefijo.slice(0, fin + 1) : sinPrefijo;
}

/** ¿El error dice que hace falta iniciar sesión? (las sesiones llegan en otra fase) */
export function pideSesion(mensaje: string): boolean {
  return ["cookies", "login", "logged-in", "log in"].some((p) => mensaje.toLowerCase().includes(p));
}
