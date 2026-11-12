// Reconocer enlaces de vídeo en texto pegado (igual que util/Enlaces.kt de Android)

const URL = /https?:\/\/[^\s<>"']+/i;

const DOMINIOS_SOPORTADOS = [
  "youtube.com", "youtu.be", "tiktok.com", "x.com", "twitter.com", "instagram.com",
  "facebook.com", "fb.watch", "fb.com", "open.spotify.com", "spotify.link",
];

/** ¿Es un enlace de Spotify (open.spotify.com, spotify.link o spotify:tipo:id)? Lo que sea
 *  exactamente lo decide Rust al leerlo. */
export function esSpotify(texto: string): boolean {
  if (/spotify:(playlist|album|track):[a-z0-9]{22}/i.test(texto)) return true;
  const url = extraerUrl(texto);
  const h = url ? host(url) : null;
  return h === "open.spotify.com" || h === "spotify.link";
}

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

function esDeYoutube(h: string | null): boolean {
  return h !== null && ["youtube.com", "youtu.be"].some((d) => h === d || h.endsWith(`.${d}`));
}

/** ¿Es un enlace que es solo una lista de YouTube (youtube.com/playlist?list=… o un álbum de
 *  YouTube Music, music.youtube.com/browse/MPREb_…)? Igual que enlaces::es_solo_lista en Rust. */
export function esListaDeYoutube(url: string): boolean {
  try {
    const u = new globalThis.URL(url);
    if (!esDeYoutube(u.hostname.toLowerCase())) return false;
    const ruta = u.pathname.replace(/\/+$/, "").toLowerCase();
    if (ruta === "/playlist") return (u.searchParams.get("list") ?? "").length > 0;
    return ruta.startsWith("/browse/mpreb_");
  } catch {
    return false;
  }
}

/** ¿Es claramente música (YouTube Music o una lista de álbum OLAK5uy_…)? Se propone «Solo audio». */
export function esMusicaDeYoutube(url: string): boolean {
  try {
    const u = new globalThis.URL(url);
    const h = u.hostname.toLowerCase();
    if (!esDeYoutube(h)) return false;
    return h === "music.youtube.com" || (u.searchParams.get("list") ?? "").startsWith("OLAK5uy_");
  } catch {
    return false;
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
  if (es("open.spotify.com", "spotify.link")) return "Spotify";
  return h.replace(/^www\./, "");
}
