// Textos de Klippo en español (por defecto) e inglés, como en la app de Android:
// inglés si Windows está en inglés, español en cualquier otro caso.

const es = {
  ajustes: "Ajustes",
  atras: "Atrás",
  pegar: "Pegar",
  borrar: "Borrar",
  descargar: "Descargar",
  campoEnlace: "Pega el enlace del vídeo",
  campoAyuda: "Enlace de YouTube, TikTok, X, Instagram o Facebook",
  noEsEnlace: "Eso no parece un enlace",
  listaVacia: "Pega un enlace para descargar el vídeo",
  listaVaciaDetalle:
    "Los vídeos se quedan en Klippo y desaparecen solos en una hora. A tu carpeta de Descargas solo llega lo que tú guardes.",
  tusVideos: "Tus vídeos",
  vistaLista: "Lista",
  vistaMosaico: "Mosaico",

  calidadTitulo: "Calidad del vídeo",
  calidadMejor: "Mejor calidad",
  calidad1080: "Hasta 1080p",
  calidad720: "Hasta 720p",
  calidad480: "Hasta 480p",
  calidadAudio: "Solo audio (M4A)",
  cancelar: "Cancelar",

  analizando: "Analizando enlace…",
  progresoEta: "{0} % · quedan {1}",
  procesando: "Uniendo imagen y sonido…",
  completada: "Completada",
  cancelada: "Cancelada",
  fallida: "Error: {0}",
  pideSesion: "Este vídeo pide haber iniciado sesión. El inicio de sesión llega en una próxima versión.",
  reintentar: "Reintentar",
  actualizarYReintentar: "Actualizar motor y reintentar",
  reproducir: "Reproducir",
  mostrarEnCarpeta: "Mostrar en la carpeta",
  eliminar: "Eliminar",
  audio: "Audio",

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

  descargas: "Descargas",
  preguntarCalidad: "Preguntar la calidad en cada vídeo",
  preguntarCalidadDetalle: "Si lo apagas, se descarga directamente con la calidad de abajo",
  calidadMarcada: "Calidad marcada por defecto",
  vista: "Mostrar los vídeos en",
  motor: "Motor de descarga",
  motorDetalle: "Actualízalo si una plataforma deja de descargar",
  motorVersiones: "yt-dlp {0} · FFmpeg {1} · Deno {2}",
  actualizandoMotor: "Actualizando…",
  motorActualizado: "Motor actualizado: yt-dlp {0}",
  motorAlDia: "El motor ya está al día",
  motorError: "No se pudo actualizar el motor: {0}",
  almacenamiento: "Almacenamiento",
  cache: "Vídeos en Klippo",
  cacheDetalle: "{0} · se borran solos al cabo de una hora",
  vaciar: "Vaciar",

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
  campoAyuda: "YouTube, TikTok, X, Instagram or Facebook link",
  noEsEnlace: "That doesn't look like a link",
  listaVacia: "Paste a link to download the video",
  listaVaciaDetalle:
    "Videos stay inside Klippo and disappear on their own after an hour. Only what you save reaches your Downloads folder.",
  tusVideos: "Your videos",
  vistaLista: "List",
  vistaMosaico: "Grid",

  calidadTitulo: "Video quality",
  calidadMejor: "Best quality",
  calidad1080: "Up to 1080p",
  calidad720: "Up to 720p",
  calidad480: "Up to 480p",
  calidadAudio: "Audio only (M4A)",
  cancelar: "Cancel",

  analizando: "Analyzing link…",
  progresoEta: "{0} % · {1} left",
  procesando: "Joining video and sound…",
  completada: "Completed",
  cancelada: "Cancelled",
  fallida: "Error: {0}",
  pideSesion: "This video requires being logged in. Logging in arrives in an upcoming version.",
  reintentar: "Retry",
  actualizarYReintentar: "Update engine and retry",
  reproducir: "Play",
  mostrarEnCarpeta: "Show in folder",
  eliminar: "Delete",
  audio: "Audio",

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

  descargas: "Downloads",
  preguntarCalidad: "Ask for the quality on every video",
  preguntarCalidadDetalle: "If you turn it off, videos download straight away with the quality below",
  calidadMarcada: "Default quality",
  vista: "Show videos as",
  motor: "Download engine",
  motorDetalle: "Update it if a platform stops downloading",
  motorVersiones: "yt-dlp {0} · FFmpeg {1} · Deno {2}",
  actualizandoMotor: "Updating…",
  motorActualizado: "Engine updated: yt-dlp {0}",
  motorAlDia: "The engine is already up to date",
  motorError: "Could not update the engine: {0}",
  almacenamiento: "Storage",
  cache: "Videos in Klippo",
  cacheDetalle: "{0} · deleted automatically after one hour",
  vaciar: "Empty",

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

/** Nombre de cada calidad en el diálogo y en Ajustes */
export function nombreCalidad(calidad: string): string {
  return (
    { mejor: t.calidadMejor, p1080: t.calidad1080, p720: t.calidad720, p480: t.calidad480, audio: t.calidadAudio }[
      calidad
    ] ?? calidad
  );
}

/** 1536000 → "1,5 MB" */
export function formatearTamano(bytes: number): string {
  const unidades = ["B", "KB", "MB", "GB"];
  let valor = bytes;
  let i = 0;
  while (valor >= 1024 && i < unidades.length - 1) {
    valor /= 1024;
    i++;
  }
  const numero = valor.toLocaleString(idioma, { maximumFractionDigits: i >= 2 ? 1 : 0 });
  return `${numero} ${unidades[i]}`;
}
