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
