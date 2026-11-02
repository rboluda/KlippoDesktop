// Arrastre con el ratón que sigue aunque el puntero salga del elemento (un tirador de 16 px
// se queda atrás enseguida). Se escucha en toda la ventana hasta soltar el botón.

/** `alMover` recibe lo que se ha movido desde el último evento, en píxeles de pantalla */
export function arrastrar(
  inicio: PointerEvent,
  alMover: (dx: number, dy: number, evento: PointerEvent) => void,
  alSoltar?: () => void,
): void {
  inicio.preventDefault();
  inicio.stopPropagation();
  let x = inicio.clientX;
  let y = inicio.clientY;
  const mover = (e: PointerEvent) => {
    if (e.pointerId !== inicio.pointerId) return;
    alMover(e.clientX - x, e.clientY - y, e);
    x = e.clientX;
    y = e.clientY;
  };
  const soltar = (e: PointerEvent) => {
    if (e.pointerId !== inicio.pointerId) return;
    window.removeEventListener("pointermove", mover);
    window.removeEventListener("pointerup", soltar);
    window.removeEventListener("pointercancel", soltar);
    alSoltar?.();
  };
  window.addEventListener("pointermove", mover);
  window.addEventListener("pointerup", soltar);
  window.addEventListener("pointercancel", soltar);
}
