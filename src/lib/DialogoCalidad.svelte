<!-- Calidad de cada descarga, con la última usada marcada -->
<script lang="ts">
  import { onMount } from "svelte";
  import { CALIDADES, type Calidad } from "./descargas.svelte";
  import { nombreCalidad, t } from "./i18n";

  let {
    marcada,
    onelegir,
    oncerrar,
  }: { marcada: Calidad; onelegir: (calidad: Calidad) => void; oncerrar: () => void } = $props();

  // svelte-ignore state_referenced_locally
  let elegida = $state<Calidad>(marcada);
  let dialogo: HTMLDialogElement;

  onMount(() => dialogo.showModal());

  function aceptar(evento: SubmitEvent) {
    evento.preventDefault();
    dialogo.close();
    onelegir(elegida);
  }
</script>

<dialog bind:this={dialogo} onclose={oncerrar}>
  <form onsubmit={aceptar}>
    <h2>{t.calidadTitulo}</h2>
    {#each CALIDADES as calidad (calidad)}
      <label class="opcion">
        <input type="radio" name="calidad" value={calidad} bind:group={elegida} />
        {nombreCalidad(calidad)}
      </label>
    {/each}
    <div class="botones">
      <button type="button" class="boton texto" onclick={() => dialogo.close()}>{t.cancelar}</button>
      <!-- svelte-ignore a11y_autofocus -->
      <button type="submit" class="boton" autofocus>{t.descargar}</button>
    </div>
  </form>
</dialog>

<style>
  dialog {
    width: 340px;
    padding: 24px 16px 16px;
    border: none;
    border-radius: 28px;
    background: var(--surface-container-high);
    color: var(--on-surface);
  }

  dialog::backdrop {
    background: rgb(0 0 0 / 0.32);
  }

  h2 {
    margin: 0 8px 12px;
    font-size: 1.3rem;
    font-weight: 500;
  }

  .opcion {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 10px 8px;
    border-radius: 12px;
    cursor: pointer;
  }

  .opcion:hover {
    background: color-mix(in srgb, var(--on-surface) 6%, transparent);
  }

  input {
    width: 18px;
    height: 18px;
    margin: 0;
    accent-color: var(--primary);
  }

  .botones {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 16px;
  }
</style>
