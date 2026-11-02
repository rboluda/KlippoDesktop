// Textos de Klippo en español (por defecto) e inglés, como en la app de Android:
// inglés si Windows está en inglés, español en cualquier otro caso.

const es = {
  ajustes: "Ajustes",
  atras: "Atrás",
  pegar: "Pegar",
  borrar: "Borrar",
  descargar: "Descargar",
  campoEnlace: "Pega el enlace del vídeo",
  descargasProximamente: "Las descargas llegan en la próxima versión de Klippo para Windows.",
  listaVacia: "Pega un enlace para descargar el vídeo",
  listaVaciaDetalle:
    "Los vídeos se quedan en Klippo y desaparecen solos en una hora. A tu carpeta de Descargas solo llega lo que tú guardes.",

  actualizaciones: "Actualizaciones",
  buscarAuto: "Buscar versiones nuevas",
  buscarAutoDetalle: "Una vez al día, al abrir Klippo",
  buscarAhora: "Buscar actualizaciones de Klippo",
  buscar: "Buscar",
  buscando: "Buscando…",
  alDia: "Tienes la última versión de Klippo",
  errorComprobar: "No se pudo comprobar si hay versión nueva",
  disponible: "Nueva versión de Klippo: {0}",
  actualizar: "Actualizar",
  descargandoActualizacion: "Descargando actualización… {0} %",
  errorActualizar: "Error al actualizar: {0}",
  cerrar: "Cerrar",

  acercaDe: "Acerca de Klippo",
  descripcion:
    "Descarga vídeos de YouTube, TikTok, X, Instagram y Facebook, recórtalos y compártelos. Sin rodeos.",
  principios:
    "Sin publicidad, sin analítica, sin cuentas. Tus vídeos no salen de Klippo salvo que tú lo pidas.",
  autor: "Hecho por rboluda",
  codigo: "Código fuente",
  miGithub: "Mi GitHub",
  licencia: "Software libre · Licencia GPL-3.0",
};

const en: typeof es = {
  ajustes: "Settings",
  atras: "Back",
  pegar: "Paste",
  borrar: "Clear",
  descargar: "Download",
  campoEnlace: "Paste the video link",
  descargasProximamente: "Downloads arrive in the next version of Klippo for Windows.",
  listaVacia: "Paste a link to download the video",
  listaVaciaDetalle:
    "Videos stay inside Klippo and disappear on their own after an hour. Only what you save reaches your Downloads folder.",

  actualizaciones: "Updates",
  buscarAuto: "Check for new versions",
  buscarAutoDetalle: "Once a day, when Klippo opens",
  buscarAhora: "Check for Klippo updates",
  buscar: "Check",
  buscando: "Checking…",
  alDia: "You have the latest version of Klippo",
  errorComprobar: "Could not check for a new version",
  disponible: "New Klippo version: {0}",
  actualizar: "Update",
  descargandoActualizacion: "Downloading update… {0} %",
  errorActualizar: "Update failed: {0}",
  cerrar: "Close",

  acercaDe: "About Klippo",
  descripcion:
    "Download videos from YouTube, TikTok, X, Instagram and Facebook, trim them and share them. No fuss.",
  principios: "No ads, no analytics, no accounts. Your videos never leave Klippo unless you ask.",
  autor: "Made by rboluda",
  codigo: "Source code",
  miGithub: "My GitHub",
  licencia: "Free software · GPL-3.0 license",
};

export const idioma: "es" | "en" =
  typeof navigator !== "undefined" && navigator.language.toLowerCase().startsWith("en") ? "en" : "es";

export const t = idioma === "en" ? en : es;

/** Rellena los huecos {0}, {1}… de un texto */
export function formato(texto: string, ...valores: (string | number)[]): string {
  return texto.replace(/\{(\d+)\}/g, (_, i) => String(valores[Number(i)] ?? ""));
}
