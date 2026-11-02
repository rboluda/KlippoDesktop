import { idioma } from "../i18n";

/** Separador decimal del idioma: 5,3 en español y 5.3 en inglés */
const SEPARADOR = idioma === "es" ? "," : ".";

/** 65300 → "1:05,3" (minutos, segundos y décimas) */
export function formatearTiempo(ms: number): string {
  const decimas = Math.round(ms / 100);
  const minutos = Math.floor(decimas / 600);
  const segundos = Math.floor((decimas % 600) / 10);
  return `${minutos}:${String(segundos).padStart(2, "0")}${SEPARADOR}${decimas % 10}`;
}

/** Segundos escritos a mano ("5,3", "5.3", "12") → milisegundos; null si no es un número */
export function leerSegundos(texto: string): number | null {
  const limpio = texto.trim().replace(",", ".");
  if (!/^\d+(\.\d*)?$/.test(limpio)) return null;
  return Math.round(parseFloat(limpio) * 10) * 100;
}
