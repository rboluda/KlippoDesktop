import { guardar, leer } from "$lib/ajustes";

// Grupo de cada lista de Spotify ya pedida ("<tipo>-<id>" → "spotify-<id>-<momento>"):
// lo que se pida después de la misma lista, mientras Klippo siga abierto, se junta con lo de antes
export const gruposSpotify = new Map<string, string>();

// ---- Listas de canciones pegadas como texto ----
// Se guardan en los ajustes (las 30 últimas) para poder volver a ellas desde su tarjeta.
// Su "enlace" es "texto:<huella del texto>": el mismo texto vuelve a la misma lista.

export const PREFIJO_TEXTO = "texto:";
const MAXIMO_GUARDADAS = 30;

interface ListaDeTexto {
  nombre: string;
  texto: string;
  /** Momento en que se usó por última vez */
  usada: number;
}

/** Texto pegado en el campo de Inicio, esperando a que lo recoja su pantalla */
export const textoPegado = { valor: "" };

/** ¿Parece una lista de canciones? Dos líneas o más con letras y sin enlaces */
export function pareceListaDeCanciones(texto: string): boolean {
  const lineas = texto.split(/\r?\n/).filter((l) => /\p{L}/u.test(l) && !/:\/\//.test(l));
  return lineas.length >= 2;
}

/** Huella FNV-1a de 32 bits, en hexadecimal */
function huella(texto: string): string {
  let h = 0x811c9dc5;
  for (let i = 0; i < texto.length; i++) {
    h ^= texto.charCodeAt(i);
    h = Math.imul(h, 0x01000193) >>> 0;
  }
  return h.toString(16).padStart(8, "0");
}

async function guardadas(): Promise<Record<string, ListaDeTexto>> {
  return leer<Record<string, ListaDeTexto>>("listasDeTexto", {});
}

/** Guarda la lista y devuelve su id. Con `id` se reemplaza esa (al editarla). */
export async function guardarListaDeTexto(texto: string, nombre: string, id?: string): Promise<string> {
  const limpio = texto.replace(/\r\n/g, "\n").trim();
  const clave = id ?? huella(limpio);
  const listas = await guardadas();
  listas[clave] = { nombre, texto: limpio, usada: Date.now() };
  const sobran = Object.entries(listas)
    .sort((a, b) => b[1].usada - a[1].usada)
    .slice(MAXIMO_GUARDADAS);
  for (const [k] of sobran) delete listas[k];
  await guardar("listasDeTexto", listas);
  return clave;
}

export async function listaDeTexto(id: string): Promise<ListaDeTexto | null> {
  return (await guardadas())[id] ?? null;
}
