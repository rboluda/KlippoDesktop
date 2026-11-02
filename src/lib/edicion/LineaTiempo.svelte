<!-- Barra de tiempo: posición de reproducción y los dos tiradores del recorte (inicio y fin).
     Pulsar la barra salta a ese punto; pulsar las horas permite escribirlas a la décima. -->
<script lang="ts">
  import { formato, t } from "../i18n";
  import { arrastrar } from "./arrastre";
  import { formatearTiempo } from "./tiempo";

  let {
    duracionMs,
    actualMs,
    inicioMs,
    finMs,
    onbuscar,
    onrecorte,
    oneditar,
  }: {
    duracionMs: number;
    actualMs: number;
    inicioMs: number;
    finMs: number;
    onbuscar: (ms: number) => void;
    onrecorte: (inicioMs: number, finMs: number) => void;
    oneditar: (cual: "inicio" | "fin") => void;
  } = $props();

  /** Lo mínimo que puede durar el recorte */
  const MINIMO_MS = 100;

  let barra: HTMLDivElement;

  const pct = (ms: number) => `${duracionMs ? (ms / duracionMs) * 100 : 0}%`;

  function msEn(evento: PointerEvent | MouseEvent): number {
    const r = barra.getBoundingClientRect();
    return Math.round(Math.min(Math.max((evento.clientX - r.left) / r.width, 0), 1) * duracionMs);
  }

  function empezar(evento: PointerEvent, que: "inicio" | "fin" | "posicion") {
    mover(evento, que);
    arrastrar(evento, (_dx, _dy, e) => mover(e, que));
  }

  function mover(evento: PointerEvent, que: "inicio" | "fin" | "posicion") {
    const ms = msEn(evento);
    if (que === "inicio") {
      const inicio = Math.min(ms, finMs - MINIMO_MS);
      onrecorte(Math.max(0, inicio), finMs);
      onbuscar(Math.max(0, inicio));
    } else if (que === "fin") {
      const fin = Math.max(ms, inicioMs + MINIMO_MS);
      onrecorte(inicioMs, Math.min(duracionMs, fin));
      onbuscar(Math.min(duracionMs, fin));
    } else {
      onbuscar(ms);
    }
  }
</script>

<div class="linea">
  <button class="hora" onclick={() => oneditar("inicio")} title={t.escribirTiempo}>
    {formato(t.recorteInicio, formatearTiempo(inicioMs))}
  </button>
  <div
    class="barra"
    bind:this={barra}
    onpointerdown={(e) => empezar(e, "posicion")}
    role="slider"
    aria-valuemin={0}
    aria-valuemax={duracionMs}
    aria-valuenow={actualMs}
    tabindex="0"
  >
    <span class="fuera" style:left="0" style:width={pct(inicioMs)}></span>
    <span class="fuera" style:left={pct(finMs)} style:right="0"></span>
    <span class="tramo" style:left={pct(inicioMs)} style:width="calc({pct(finMs)} - {pct(inicioMs)})"></span>
    <span class="cabezal" style:left={pct(actualMs)}></span>
    <span
      class="tirador"
      style:left={pct(inicioMs)}
      onpointerdown={(e) => empezar(e, "inicio")}
      role="presentation"
    ></span>
    <span
      class="tirador"
      style:left={pct(finMs)}
      onpointerdown={(e) => empezar(e, "fin")}
      role="presentation"
    ></span>
  </div>
  <button class="hora" onclick={() => oneditar("fin")} title={t.escribirTiempo}>
    {formato(t.recorteFin, formatearTiempo(finMs))}
  </button>
</div>

<style>
  .linea {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .hora {
    flex: none;
    min-width: 110px;
    height: 32px;
    padding: 0 10px;
    border: 1px solid var(--outline-variant);
    border-radius: 8px;
    background: var(--surface-container-high);
    font-variant-numeric: tabular-nums;
    cursor: pointer;
  }

  .hora:hover {
    border-color: var(--primary);
  }

  .barra {
    position: relative;
    flex: 1;
    height: 40px;
    border-radius: 8px;
    background: var(--surface-container-highest);
    cursor: pointer;
    touch-action: none;
  }

  .fuera {
    position: absolute;
    top: 0;
    bottom: 0;
    background: rgb(0 0 0 / 0.18);
  }

  .tramo {
    position: absolute;
    top: 0;
    bottom: 0;
    border-top: 3px solid var(--primary);
    border-bottom: 3px solid var(--primary);
    background: color-mix(in srgb, var(--primary) 18%, transparent);
  }

  .cabezal {
    position: absolute;
    top: -4px;
    bottom: -4px;
    width: 2px;
    margin-left: -1px;
    background: var(--on-surface);
    pointer-events: none;
  }

  .tirador {
    position: absolute;
    top: -2px;
    bottom: -2px;
    width: 12px;
    margin-left: -6px;
    border-radius: 4px;
    background: var(--primary);
    cursor: ew-resize;
  }
</style>
