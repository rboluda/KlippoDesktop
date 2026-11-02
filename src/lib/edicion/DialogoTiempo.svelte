<!-- Escribir a mano el inicio o el fin del recorte: minutos y segundos con décimas -->
<script lang="ts">
  import Dialogo from "../Dialogo.svelte";
  import { formato, t } from "../i18n";
  import { formatearTiempo, leerSegundos } from "./tiempo";

  let {
    cual,
    valorMs,
    minimoMs,
    maximoMs,
    onaceptar,
    oncerrar,
  }: {
    cual: "inicio" | "fin";
    valorMs: number;
    minimoMs: number;
    maximoMs: number;
    onaceptar: (ms: number) => void;
    oncerrar: () => void;
  } = $props();

  // svelte-ignore state_referenced_locally
  let minutos = $state(String(Math.floor(valorMs / 60000)));
  // svelte-ignore state_referenced_locally
  let segundos = $state(formatearTiempo(valorMs % 60000).split(":")[1]);
  let error = $state("");

  function aceptar() {
    const min = /^\d*$/.test(minutos.trim()) ? Number(minutos.trim() || 0) : null;
    const seg = leerSegundos(segundos);
    if (min === null || seg === null) {
      error = t.tiempoErrorFormato;
      return false;
    }
    const ms = min * 60000 + seg;
    if (ms < minimoMs || ms > maximoMs) {
      error = formato(t.tiempoErrorRango, formatearTiempo(minimoMs), formatearTiempo(maximoMs));
      return false;
    }
    onaceptar(ms);
  }
</script>

<Dialogo titulo={cual === "inicio" ? t.tiempoTituloInicio : t.tiempoTituloFin} onaceptar={aceptar} {oncerrar}>
  <div class="campos">
    <label>
      <!-- svelte-ignore a11y_autofocus -->
      <input type="text" inputmode="numeric" bind:value={minutos} oninput={() => (error = "")} autofocus />
      <span>{t.tiempoMinutos}</span>
    </label>
    <span class="dos-puntos">:</span>
    <label>
      <input type="text" inputmode="decimal" bind:value={segundos} oninput={() => (error = "")} />
      <span>{t.tiempoSegundos}</span>
    </label>
  </div>
  <p class:error>{error || t.tiempoAyuda}</p>
</Dialogo>

<style>
  .campos {
    display: flex;
    align-items: flex-start;
    gap: 8px;
  }

  label {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 0.85rem;
    color: var(--on-surface-variant);
  }

  .dos-puntos {
    padding-top: 10px;
    font-size: 1.4rem;
  }

  p {
    margin: 12px 0 0;
    font-size: 0.87rem;
    color: var(--on-surface-variant);
  }

  p.error {
    color: var(--error);
  }
</style>
