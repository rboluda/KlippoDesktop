<!-- Un texto o emoji sobre el vídeo: se arrastra, se cambia de tamaño con la rueda del ratón o
     el tirador, y se quita con × o con Supr -->
<script lang="ts">
  import { escalar, moverSuperpuesto, type Superpuesto } from "./modelo";
  import { dibujar } from "./render";

  let {
    elemento,
    anchoPx,
    altoPx,
    anchoVideo,
    altoVideo,
    seleccionado,
    activo = true,
    onseleccionar,
    onchange,
    onquitar,
  }: {
    elemento: Superpuesto;
    anchoPx: number;
    altoPx: number;
    anchoVideo: number;
    altoVideo: number;
    seleccionado: boolean;
    /** Con el encuadre o el pixelado en uso, los textos no estorban al arrastrar */
    activo?: boolean;
    onseleccionar: () => void;
    onchange: (s: Superpuesto) => void;
    onquitar: () => void;
  } = $props();

  // Se dibuja al tamaño real del vídeo (como saldrá) y se muestra a escala
  const dibujo = $derived(dibujar(elemento, anchoVideo, altoVideo));
  const imagen = $derived(dibujo.lienzo.toDataURL());
  const escala = $derived(altoPx / altoVideo);

  let arrastre: { modo: "mover" | "tamano"; x: number; y: number } | null = null;

  function empezar(evento: PointerEvent, modo: "mover" | "tamano") {
    evento.stopPropagation();
    onseleccionar();
    (evento.currentTarget as Element).setPointerCapture(evento.pointerId);
    arrastre = { modo, x: evento.clientX, y: evento.clientY };
  }

  function arrastrar(evento: PointerEvent) {
    if (!arrastre) return;
    const dx = evento.clientX - arrastre.x;
    const dy = evento.clientY - arrastre.y;
    arrastre.x = evento.clientX;
    arrastre.y = evento.clientY;
    if (arrastre.modo === "mover") {
      onchange(moverSuperpuesto(elemento, dx / anchoPx, dy / altoPx));
    } else {
      // El tirador de la esquina: alejarse del centro agranda
      const mitad = (dibujo.lienzo.height * escala) / 2;
      onchange(escalar(elemento, Math.max(0.5, (mitad + (dx + dy) / 2) / mitad)));
    }
  }

  function rueda(evento: WheelEvent) {
    evento.preventDefault();
    onseleccionar();
    onchange(escalar(elemento, evento.deltaY < 0 ? 1.08 : 1 / 1.08));
  }

  const soltar = () => (arrastre = null);
</script>

<div
  class="superpuesto"
  class:seleccionado
  class:inactivo={!activo}
  style:left="{elemento.x * 100}%"
  style:top="{elemento.y * 100}%"
  style:width="{dibujo.lienzo.width * escala}px"
  style:height="{dibujo.lienzo.height * escala}px"
  onpointerdown={(e) => empezar(e, "mover")}
  onpointermove={arrastrar}
  onpointerup={soltar}
  onwheel={rueda}
  role="presentation"
>
  <img src={imagen} alt={elemento.texto} draggable="false" />
  {#if seleccionado}
    <button class="quitar" onpointerdown={(e) => e.stopPropagation()} onclick={onquitar} aria-label="×">×</button>
    <span
      class="tirador"
      onpointerdown={(e) => empezar(e, "tamano")}
      onpointermove={arrastrar}
      onpointerup={soltar}
      role="presentation"
    ></span>
  {/if}
</div>

<style>
  .superpuesto {
    position: absolute;
    transform: translate(-50%, -50%);
    cursor: move;
    touch-action: none;
  }

  .superpuesto img {
    width: 100%;
    height: 100%;
    display: block;
    user-select: none;
  }

  .inactivo {
    pointer-events: none;
  }

  .seleccionado {
    outline: 2px dashed #fff;
    outline-offset: 4px;
  }

  .quitar {
    position: absolute;
    top: -16px;
    right: -16px;
    width: 24px;
    height: 24px;
    border: none;
    border-radius: 50%;
    background: rgb(0 0 0 / 0.75);
    color: #fff;
    font-size: 16px;
    line-height: 1;
    cursor: pointer;
  }

  .tirador {
    position: absolute;
    right: -12px;
    bottom: -12px;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: #fff;
    box-shadow: 0 0 0 2px var(--primary);
    cursor: nwse-resize;
  }
</style>
