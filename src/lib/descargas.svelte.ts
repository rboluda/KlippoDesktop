// Lista de descargas: la lleva el lado de Rust y avisa de cada cambio con el evento "descargas"
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { startDrag } from "@crabnebula/tauri-plugin-drag";
import { formato, t } from "./i18n";
import { mostrarMensaje } from "./mensajes.svelte";

/** "audio" es solo audio en M4A (el nombre de siempre, para no perder lo ya guardado); "mp3", en MP3 */
export type Calidad = "mejor" | "p1080" | "p720" | "p480" | "audio" | "mp3";
export const CALIDADES: Calidad[] = ["mejor", "p1080", "p720", "p480", "audio", "mp3"];

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
  /** Copia en Descargas\Klippo, si se guardó */
  guardado: string | null;
  /** Vídeos de una misma lista o carrusel: id de la descarga original y nombre de la lista */
  grupo: string | null;
  lista: string | null;
  /** Mientras se baja una lista: por qué elemento va y cuántos hay ("4 de 10") */
  elemento: number | null;
  total: number | null;
}

/** Una entrada de Inicio: una descarga suelta o una lista con todos sus vídeos */
export type Entrada = { tipo: "suelta"; descarga: Descarga } | { tipo: "lista"; grupo: string; elementos: Descarga[] };

/** Junta los vídeos de cada lista en una sola entrada, donde aparece el primero */
export function agrupar(lista: Descarga[]): Entrada[] {
  const entradas: Entrada[] = [];
  const grupos = new Map<string, Descarga[]>();
  for (const d of lista) {
    if (!d.grupo || d.estado.tipo !== "completada") {
      entradas.push({ tipo: "suelta", descarga: d });
      continue;
    }
    let elementos = grupos.get(d.grupo);
    if (!elementos) {
      elementos = [];
      grupos.set(d.grupo, elementos);
      entradas.push({ tipo: "lista", grupo: d.grupo, elementos });
    }
    elementos.push(d);
  }
  // Una "lista" que se ha quedado con un solo vídeo (p. ej. tras borrar los demás) va suelta
  return entradas.map((e) => (e.tipo === "lista" && e.elementos.length === 1 ? { tipo: "suelta", descarga: e.elementos[0] } : e));
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
export const mostrarGuardado = (id: string) => invoke("mostrar_guardado", { id });

// ---- Guardar y compartir: cada acción avisa con un mensaje de cómo ha ido ----

async function intentar(accion: () => Promise<unknown>): Promise<void> {
  try {
    await accion();
  } catch (e) {
    mostrarMensaje({ texto: formato(t.noSePudo, String(e)) });
  }
}

export const guardar = (d: Descarga) =>
  intentar(async () => {
    await invoke<string>("guardar_descarga", { id: d.id });
    mostrarMensaje({ texto: t.guardado, accion: { texto: t.mostrar, hacer: () => mostrarGuardado(d.id) } });
  });

export const copiar = (d: Descarga) =>
  intentar(async () => {
    await invoke("copiar_descarga", { id: d.id });
    mostrarMensaje({ texto: d.esAudio ? t.copiadoAudio : t.copiado });
  });

export const whatsapp = (d: Descarga) =>
  intentar(async () => {
    await invoke<boolean>("whatsapp_descarga", { id: d.id });
    mostrarMensaje({ texto: t.whatsappAbierto });
  });

export const compartir = (d: Descarga) => intentar(() => invoke("compartir_descarga", { id: d.id }));

// ---- La lista entera ----

export const guardarLista = (grupo: string, nombre: string) =>
  intentar(async () => {
    const carpeta = await invoke<string>("guardar_lista", { grupo });
    mostrarMensaje({
      texto: formato(t.listaGuardada, nombre),
      accion: { texto: t.mostrar, hacer: () => invoke("abrir_carpeta", { ruta: carpeta }) },
    });
  });

export const copiarLista = (grupo: string, cuantos: number) =>
  intentar(async () => {
    await invoke("copiar_lista", { grupo });
    mostrarMensaje({ texto: formato(t.listaCopiada, cuantos) });
  });

export const whatsappLista = (grupo: string) =>
  intentar(async () => {
    await invoke<boolean>("whatsapp_lista", { grupo });
    mostrarMensaje({ texto: t.whatsappAbiertoLista });
  });

export const compartirLista = (grupo: string) => intentar(() => invoke("compartir_lista", { grupo }));
export const eliminarLista = (grupo: string) => invoke("eliminar_lista", { grupo });

/** Una imagen mínima para el arrastre cuando no hay miniatura (PNG transparente de 1×1) */
const SIN_IMAGEN =
  "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=";

/** Arrastrar el archivo fuera de Klippo: a WhatsApp, a un correo, a una carpeta… */
export function arrastrarFuera(evento: DragEvent, d: Descarga): void {
  evento.preventDefault();
  if (!d.archivo) return;
  intentar(() => startDrag({ item: [d.archivo!], icon: d.miniatura ?? SIN_IMAGEN }));
}

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
