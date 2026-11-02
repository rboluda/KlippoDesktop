// Mensaje breve abajo de la ventana (como el Snackbar de Android), con una acción opcional
export interface Mensaje {
  texto: string;
  accion?: { texto: string; hacer: () => void };
}

export const mensajes = $state({ actual: null as Mensaje | null });

let temporizador: ReturnType<typeof setTimeout> | undefined;

export function mostrarMensaje(mensaje: Mensaje): void {
  mensajes.actual = mensaje;
  clearTimeout(temporizador);
  temporizador = setTimeout(() => (mensajes.actual = null), mensaje.accion ? 8000 : 5000);
}

export function cerrarMensaje(): void {
  clearTimeout(temporizador);
  mensajes.actual = null;
}
