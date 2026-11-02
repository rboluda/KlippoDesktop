// Sesiones de Instagram, X y Facebook: estado y acciones (la ventana la abre Rust)
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { formato, t } from "./i18n";
import { mostrarMensaje } from "./mensajes.svelte";

export type Plataforma = "instagram" | "x" | "facebook";

export interface EstadoSesion {
  plataforma: Plataforma;
  nombre: string;
  iniciada: boolean;
}

export const sesiones = $state({ lista: [] as EstadoSesion[] });

let escuchando = false;

export async function cargarSesiones(): Promise<void> {
  sesiones.lista = await invoke<EstadoSesion[]>("estado_sesiones");
  if (escuchando) return;
  escuchando = true;
  // Al detectar la sesión, la ventana se cierra sola y llega el estado nuevo
  await listen<EstadoSesion[]>("sesiones", (e) => {
    const antes = sesiones.lista;
    sesiones.lista = e.payload;
    const nueva = e.payload.find((s) => s.iniciada && !antes.find((a) => a.plataforma === s.plataforma)?.iniciada);
    if (nueva) mostrarMensaje({ texto: formato(t.sesionGuardada, nueva.nombre) });
  });
}

export async function iniciarSesion(plataforma: Plataforma): Promise<void> {
  await cargarSesiones();
  await invoke("iniciar_sesion", { plataforma });
}

export async function cerrarSesion(plataforma: Plataforma): Promise<void> {
  sesiones.lista = await invoke<EstadoSesion[]>("cerrar_sesion", { plataforma });
}

/** Plataforma con sesión posible de un enlace (Instagram, X o Facebook) */
export const plataformaDe = (url: string) => invoke<Plataforma | null>("plataforma_de", { url });
