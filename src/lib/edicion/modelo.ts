// Parámetros del editor, como en Klippo para Android (edicion/*.kt). Todas las posiciones
// son relativas (0..1) al fotograma completo, tal como se ve, para que no dependan del tamaño
// de la ventana.

export interface Recorte {
  inicioMs: number;
  finMs: number;
}

/** Formatos de encuadre; la proporción es ancho/alto en píxeles (null = libre) */
export const FORMATOS = {
  vertical916: { etiqueta: "9:16", proporcion: 9 / 16 },
  cuadrado: { etiqueta: "1:1", proporcion: 1 },
  vertical45: { etiqueta: "4:5", proporcion: 4 / 5 },
  libre: { etiqueta: "", proporcion: null },
} as const;
export type Formato = keyof typeof FORMATOS;

export interface Zona {
  x: number;
  y: number;
  ancho: number;
  alto: number;
}

export interface Encuadre extends Zona {
  formato: Formato;
}

/** Lado mínimo del encuadre (10 % del vídeo) y de una zona pixelada (una matrícula es pequeña) */
export const MINIMO_ENCUADRE = 0.1;
export const MINIMO_PIXELADO = 0.04;

const entre = (valor: number, min: number, max: number) => Math.min(Math.max(valor, min), max);

export function mover<T extends Zona>(z: T, dx: number, dy: number): T {
  return { ...z, x: entre(z.x + dx, 0, 1 - z.ancho), y: entre(z.y + dy, 0, 1 - z.alto) };
}

/**
 * Arrastra una esquina (0 arriba-izq, 1 arriba-der, 2 abajo-izq, 3 abajo-der) dejando fija la
 * opuesta. Con `proporcion` (ancho/alto en píxeles) se mantiene la forma.
 */
export function redimensionar<T extends Zona>(
  z: T,
  esquina: number,
  dx: number,
  dy: number,
  aspectoVideo: number,
  proporcion: number | null,
  minimo: number,
): T {
  const izquierda = esquina % 2 === 0;
  const arriba = esquina < 2;
  const anclaX = izquierda ? z.x + z.ancho : z.x;
  const anclaY = arriba ? z.y + z.alto : z.y;
  const maxAncho = izquierda ? anclaX : 1 - anclaX;
  const maxAlto = arriba ? anclaY : 1 - anclaY;
  let ancho = entre(izquierda ? z.ancho - dx : z.ancho + dx, minimo, maxAncho);
  let alto: number;
  if (proporcion === null) {
    alto = entre(arriba ? z.alto - dy : z.alto + dy, minimo, maxAlto);
  } else {
    // Proporción en coordenadas relativas: (ancho/alto en píxeles) / aspecto del vídeo
    const relativa = proporcion / aspectoVideo;
    alto = ancho / relativa;
    if (alto > maxAlto) {
      alto = maxAlto;
      ancho = alto * relativa;
    }
  }
  return {
    ...z,
    x: izquierda ? anclaX - ancho : anclaX,
    y: arriba ? anclaY - alto : anclaY,
    ancho,
    alto,
  };
}

/** El mayor encuadre del formato que cabe en el vídeo, centrado */
export function encuadreCentrado(formato: Formato, aspectoVideo: number): Encuadre {
  const proporcion = FORMATOS[formato].proporcion;
  if (proporcion === null) return { formato, x: 0.1, y: 0.1, ancho: 0.8, alto: 0.8 };
  const relativa = proporcion / aspectoVideo;
  let ancho = 1;
  let alto = ancho / relativa;
  if (alto > 1) {
    alto = 1;
    ancho = alto * relativa;
  }
  return { formato, x: (1 - ancho) / 2, y: (1 - alto) / 2, ancho, alto };
}

/** Zona pixelada nueva: un cuadrado centrado de un tercio del lado corto */
export function zonaNueva(aspectoVideo: number): Zona & { id: number } {
  const alto = aspectoVideo >= 1 ? 0.33 : 0.33 * aspectoVideo;
  const ancho = alto / aspectoVideo;
  return { id: Date.now() + Math.random(), x: (1 - ancho) / 2, y: (1 - alto) / 2, ancho, alto };
}

/** Estilos del texto (los emojis no llevan) */
export type EstiloTexto = "contorno" | "fondo";

/**
 * Emoji o texto sobre el vídeo: (x, y) es el centro y `tamano` el tamaño de letra, relativos
 * al fotograma. Se muestra durante todo el vídeo.
 */
export interface Superpuesto {
  id: number;
  texto: string;
  x: number;
  y: number;
  tamano: number;
  estilo: EstiloTexto | null;
}

export const TAMANO_EMOJI = 0.15;
/** El texto empieza más pequeño que un emoji: suele ser más largo */
export const TAMANO_TEXTO = 0.07;
const TAMANO_MINIMO = 0.03;
const TAMANO_MAXIMO = 0.8;

export function escalar(s: Superpuesto, factor: number): Superpuesto {
  return { ...s, tamano: entre(s.tamano * factor, TAMANO_MINIMO, TAMANO_MAXIMO) };
}

export function moverSuperpuesto(s: Superpuesto, dx: number, dy: number): Superpuesto {
  return { ...s, x: entre(s.x + dx, 0, 1), y: entre(s.y + dy, 0, 1) };
}

/** Todo lo que el usuario ha elegido en el editor */
export interface ParametrosEdicion {
  recorte: Recorte | null;
  encuadre: Encuadre | null;
  superpuestos: Superpuesto[];
  pixelados: (Zona & { id: number })[];
  silenciar: boolean;
}

export function sinCambios(): ParametrosEdicion {
  return { recorte: null, encuadre: null, superpuestos: [], pixelados: [], silenciar: false };
}

export function hayCambios(p: ParametrosEdicion): boolean {
  return (
    p.recorte !== null || p.encuadre !== null || p.superpuestos.length > 0 || p.pixelados.length > 0 || p.silenciar
  );
}

/** Texto estable que identifica estos parámetros (si ya se exportó igual, se reutiliza) */
export function clave(p: ParametrosEdicion): string {
  const n = (v: number) => v.toFixed(4);
  return JSON.stringify([
    p.recorte && [p.recorte.inicioMs, p.recorte.finMs],
    p.encuadre && [p.encuadre.formato, n(p.encuadre.x), n(p.encuadre.y), n(p.encuadre.ancho), n(p.encuadre.alto)],
    p.superpuestos.map((s) => [s.texto, s.estilo, n(s.x), n(s.y), n(s.tamano)]),
    p.pixelados.map((z) => [n(z.x), n(z.y), n(z.ancho), n(z.alto)]),
    p.silenciar,
  ]);
}

/** Lo editado de cada vídeo se conserva mientras Klippo esté abierto */
export const ediciones = new Map<string, ParametrosEdicion>();

/** Emojis más usados para elegir de un clic (los mismos que en Android) */
export const FRECUENTES = [
  "😂", "😍", "🔥", "❤️", "👍", "🙌", "😎", "🎉", "💯", "😱", "🤣", "👀", "✨", "🙈", "💀", "🥳",
];

export type EstadoExportacion =
  | { fase: "exportando"; progreso: number }
  | { fase: "listo"; ruta: string }
  | { fase: "error"; mensaje: string };
