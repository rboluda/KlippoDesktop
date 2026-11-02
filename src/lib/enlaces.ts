// Reconocer enlaces de vídeo en texto pegado (igual que util/Enlaces.kt de Android)

const URL = /https?:\/\/[^\s<>"']+/i;

const DOMINIOS_SOPORTADOS = [
  "youtube.com", "youtu.be", "tiktok.com", "x.com", "twitter.com", "instagram.com",
  "facebook.com", "fb.watch", "fb.com",
];

/** Las apps suelen compartir "Mira este vídeo https://...": se queda solo la URL */
export function extraerUrl(texto: string | null | undefined): string | null {
  const encontrada = texto?.match(URL)?.[0];
  return encontrada ? encontrada.replace(/[.,)\]]+$/, "") : null;
}

function host(url: string): string | null {
  try {
    return new globalThis.URL(url).hostname.toLowerCase();
  } catch {
    return null;
  }
}

/** ¿Es de una de las plataformas conocidas? */
export function esSoportada(url: string): boolean {
  const h = host(url);
  return h !== null && DOMINIOS_SOPORTADOS.some((d) => h === d || h.endsWith(`.${d}`));
}

/** Nombre de la plataforma para enseñarlo en la tarjeta */
export function plataforma(url: string): string {
  const h = host(url) ?? "";
  const es = (...dominios: string[]) => dominios.some((d) => h === d || h.endsWith(`.${d}`));
  if (es("youtube.com", "youtu.be")) return "YouTube";
  if (es("tiktok.com")) return "TikTok";
  if (es("x.com", "twitter.com")) return "X";
  if (es("instagram.com")) return "Instagram";
  if (es("facebook.com", "fb.watch", "fb.com")) return "Facebook";
  return h.replace(/^www\./, "");
}
