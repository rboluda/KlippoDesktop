// Motor de descarga (yt-dlp, FFmpeg y Deno): versiones y "Actualizar motor"
import { invoke } from "@tauri-apps/api/core";
import { formato, t } from "./i18n";

export interface Versiones {
  "yt-dlp": string;
  ffmpeg: string;
  deno: string;
}

type Resultado = { resultado: "actualizado"; version: string } | { resultado: "alDia" };

export const motor = $state({
  versiones: null as Versiones | null,
  actualizando: false,
  /** Lo que pasó en la última actualización, para enseñarlo */
  mensaje: "",
});

/** Días desde la versión de yt-dlp (su número es la fecha: 2026.10.26) */
export function diasMotor(): number | null {
  const partes = motor.versiones?.["yt-dlp"].split(".").map(Number);
  if (!partes || partes.length < 3 || partes.some(isNaN)) return null;
  const fecha = new Date(partes[0], partes[1] - 1, partes[2]);
  return Math.floor((Date.now() - fecha.getTime()) / 86_400_000);
}

/** Como en Android: a partir de un mes, se sugiere actualizar */
export const DIAS_MOTOR_ANTIGUO = 30;

export async function cargarVersiones(): Promise<void> {
  try {
    motor.versiones = await invoke<Versiones>("info_motor");
  } catch (e) {
    motor.mensaje = String(e);
  }
}

/** Devuelve true si el motor quedó actualizado o ya lo estaba */
export async function actualizarMotor(): Promise<boolean> {
  if (motor.actualizando) return false;
  motor.actualizando = true;
  motor.mensaje = "";
  try {
    const r = await invoke<Resultado>("actualizar_motor");
    motor.mensaje = r.resultado === "actualizado" ? formato(t.motorActualizado, r.version) : t.motorAlDia;
    await cargarVersiones();
    return true;
  } catch (e) {
    motor.mensaje = formato(t.motorError, String(e));
    return false;
  } finally {
    motor.actualizando = false;
  }
}
