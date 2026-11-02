<!-- Rectángulo que se arrastra y se ajusta por las esquinas: el encuadre (oscurece lo que se
     queda fuera) o una zona pixelada (difumina lo de debajo en la vista previa) -->
<script lang="ts">
  import { arrastrar } from "./arrastre";
  import { mover, redimensionar, type Zona } from "./modelo";

  let {
    zona,
    anchoPx,
    altoPx,
    aspecto,
    proporcion = null,
    minimo,
    tipo,
    activa,
    onchange,
    onquitar,
    titulo = "",
  }: {
    zona: Zona;
    anchoPx: number;
    altoPx: number;
    aspecto: number;
    proporcion?: number | null;
    minimo: number;
    tipo: "encuadre" | "pixelado";
    activa: boolean;
    onchange: (z: Zona) => void;
    onquitar?: () => void;
    titulo?: string;
  } = $props();

  /** `esquina` -1 = se mueve la zona entera */
  function empezar(evento: PointerEvent, esquina: number) {
    if (!activa) return;
    // La zona cambia en cada paso: se trabaja sobre la última versión, no sobre la del inicio
    let actual = zona;
    arrastrar(evento, (px, py) => {
      const dx = px / anchoPx;
      const dy = py / altoPx;
      actual = esquina < 0 ? mover(actual, dx, dy) : redimensionar(actual, esquina, dx, dy, aspecto, proporcion, minimo);
      onchange(actual);
    });
  }
</script>

<div
  class="zona {tipo}"
  class:activa
  style:left="{zona.x * 100}%"
  style:top="{zona.y * 100}%"
  style:width="{zona.ancho * 100}%"
  style:height="{zona.alto * 100}%"
  title={titulo}
  onpointerdown={(e) => empezar(e, -1)}
  role="presentation"
>
  {#if activa}
    {#each [0, 1, 2, 3] as esquina (esquina)}
      <span
        class="esquina e{esquina}"
        onpointerdown={(e) => empezar(e, esquina)}
        role="presentation"
      ></span>
    {/each}
    {#if onquitar}
      <button class="quitar" onpointerdown={(e) => e.stopPropagation()} onclick={onquitar} aria-label="×">×</button>
    {/if}
  {/if}
</div>

<style>
  .zona {
    position: absolute;
    box-sizing: border-box;
    pointer-events: none;
  }

  .zona.activa {
    pointer-events: auto;
    cursor: move;
    touch-action: none;
  }

  /* Encuadre: lo de fuera se oscurece con una sombra enorme */
  .encuadre {
    border: 2px solid #fff;
    box-shadow: 0 0 0 9999px rgb(0 0 0 / 0.55);
  }

  .encuadre:not(.activa) {
    border-color: rgb(255 255 255 / 0.5);
  }

  /* Pixelado: en la vista previa se difumina; el vídeo exportado lleva bloques */
  .pixelado {
    backdrop-filter: blur(10px);
    background: repeating-conic-gradient(rgb(255 255 255 / 0.08) 0 25%, transparent 0 50%) 0 0 / 12px 12px;
  }

  .pixelado.activa {
    outline: 2px dashed #fff;
  }

  .esquina {
    position: absolute;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: #fff;
    box-shadow: 0 0 0 2px var(--primary);
  }

  .e0 {
    left: -8px;
    top: -8px;
    cursor: nwse-resize;
  }
  .e1 {
    right: -8px;
    top: -8px;
    cursor: nesw-resize;
  }
  .e2 {
    left: -8px;
    bottom: -8px;
    cursor: nesw-resize;
  }
  .e3 {
    right: -8px;
    bottom: -8px;
    cursor: nwse-resize;
  }

  .quitar {
    position: absolute;
    top: 4px;
    right: 4px;
    width: 24px;
    height: 24px;
    border: none;
    border-radius: 50%;
    background: rgb(0 0 0 / 0.7);
    color: #fff;
    font-size: 16px;
    line-height: 1;
    cursor: pointer;
  }
</style>
