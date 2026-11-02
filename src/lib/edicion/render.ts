// Textos y emojis dibujados en un lienzo al tamaño real del vídeo. La misma imagen sirve de
// vista previa en el editor y de PNG para FFmpeg, así lo que se ve es lo que sale.
import type { Superpuesto } from "./modelo";

// Proporciones respecto al tamaño de letra (las mismas que en Android)
const GROSOR_CONTORNO = 0.14;
const MARGEN_FONDO = 0.3;
const RADIO_FONDO = 0.25;
const OPACIDAD_FONDO = 0.6;

const FUENTE_TEXTO = '"Segoe UI Variable Display", "Segoe UI", system-ui, sans-serif';
const FUENTE_EMOJI = '"Segoe UI Emoji", "Segoe UI Symbol", sans-serif';

export interface Dibujo {
  lienzo: HTMLCanvasElement;
  /** Esquina superior izquierda en píxeles del vídeo */
  x: number;
  y: number;
}

/** Tamaño de letra = tamano × alto del vídeo */
export function dibujar(s: Superpuesto, anchoVideo: number, altoVideo: number): Dibujo {
  const tamanoLetra = Math.max(4, s.tamano * altoVideo);
  const lienzo = s.estilo ? dibujarTexto(s.texto, tamanoLetra, s.estilo) : dibujarEmoji(s.texto, tamanoLetra);
  return {
    lienzo,
    x: Math.round(s.x * anchoVideo - lienzo.width / 2),
    y: Math.round(s.y * altoVideo - lienzo.height / 2),
  };
}

/** PNG en base64 (sin la cabecera data:) para la exportación */
export function png(dibujo: Dibujo): string {
  return dibujo.lienzo.toDataURL("image/png").split(",")[1];
}

/** Emoji recortado a su silueta */
function dibujarEmoji(texto: string, tamanoLetra: number): HTMLCanvasElement {
  const medida = contexto(document.createElement("canvas"), `${tamanoLetra}px ${FUENTE_EMOJI}`).measureText(texto);
  const ancho = Math.max(1, Math.ceil(medida.actualBoundingBoxLeft + medida.actualBoundingBoxRight));
  const alto = Math.max(1, Math.ceil(medida.actualBoundingBoxAscent + medida.actualBoundingBoxDescent));
  const lienzo = Object.assign(document.createElement("canvas"), { width: ancho, height: alto });
  const c = contexto(lienzo, `${tamanoLetra}px ${FUENTE_EMOJI}`);
  c.fillText(texto, medida.actualBoundingBoxLeft, medida.actualBoundingBoxAscent);
  return lienzo;
}

/** Texto en negrita, blanco, con contorno negro o sobre una franja oscura redondeada */
function dibujarTexto(texto: string, tamanoLetra: number, estilo: "contorno" | "fondo"): HTMLCanvasElement {
  const fuente = `bold ${tamanoLetra}px ${FUENTE_TEXTO}`;
  const medida = contexto(document.createElement("canvas"), fuente).measureText(texto);
  const altoTexto = medida.fontBoundingBoxAscent + medida.fontBoundingBoxDescent;
  const margen = tamanoLetra * (estilo === "contorno" ? GROSOR_CONTORNO : MARGEN_FONDO);
  const ancho = Math.max(1, Math.ceil(medida.width + margen * 2));
  const alto = Math.max(1, Math.ceil(altoTexto + margen * 2));
  const lienzo = Object.assign(document.createElement("canvas"), { width: ancho, height: alto });
  const c = contexto(lienzo, fuente);
  const baseX = margen;
  const baseY = margen + medida.fontBoundingBoxAscent;
  if (estilo === "contorno") {
    c.strokeStyle = "#000";
    c.lineWidth = tamanoLetra * GROSOR_CONTORNO;
    c.lineJoin = "round";
    c.strokeText(texto, baseX, baseY);
  } else {
    c.fillStyle = `rgba(0, 0, 0, ${OPACIDAD_FONDO})`;
    c.beginPath();
    c.roundRect(0, 0, ancho, alto, tamanoLetra * RADIO_FONDO);
    c.fill();
  }
  c.fillStyle = "#fff";
  c.fillText(texto, baseX, baseY);
  return lienzo;
}

function contexto(lienzo: HTMLCanvasElement, fuente: string): CanvasRenderingContext2D {
  const c = lienzo.getContext("2d")!;
  c.font = fuente;
  c.textBaseline = "alphabetic";
  return c;
}
