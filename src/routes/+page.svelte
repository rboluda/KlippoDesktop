<!-- Inicio: campo del enlace y, debajo, los vídeos descargados -->
<script lang="ts">
  import Icono from "$lib/Icono.svelte";
  import { t } from "$lib/i18n";

  let enlace = $state("");
</script>

<section class="entrada">
  <div class="campo">
    <Icono nombre="enlace" />
    <input type="url" bind:value={enlace} placeholder={t.campoEnlace} spellcheck="false" />
    {#if enlace}
      <button class="boton-icono" title={t.borrar} aria-label={t.borrar} onclick={() => (enlace = "")}>
        <Icono nombre="cerrar" tamano={20} />
      </button>
    {/if}
  </div>
  <button class="boton" disabled>{t.descargar}</button>
</section>
<p class="nota">{t.descargasProximamente}</p>

<section class="vacia">
  <img src="/klippo.png" alt="" width="96" height="96" />
  <h2>{t.listaVacia}</h2>
  <p>{t.listaVaciaDetalle}</p>
</section>

<style>
  .entrada {
    display: flex;
    gap: 12px;
    align-items: center;
  }

  .campo {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 10px;
    height: 52px;
    padding: 0 6px 0 16px;
    border-radius: 26px;
    background: var(--surface-container-high);
    color: var(--on-surface-variant);
  }

  .campo:focus-within {
    outline: 2px solid var(--primary);
  }

  .campo input {
    flex: 1;
    min-width: 0;
    border: none;
    outline: none;
    background: transparent;
    color: var(--on-surface);
    user-select: text;
  }

  .entrada .boton {
    height: 52px;
    border-radius: 26px;
  }

  .nota {
    margin: 8px 16px 0;
    font-size: 0.85rem;
    color: var(--on-surface-variant);
  }

  .vacia {
    max-width: 440px;
    margin: 12vh auto 0;
    text-align: center;
    color: var(--on-surface-variant);
  }

  .vacia img {
    border-radius: 22px;
    opacity: 0.9;
  }

  .vacia h2 {
    margin: 20px 0 8px;
    font-size: 1.15rem;
    color: var(--on-surface);
  }

  .vacia p {
    margin: 0;
  }
</style>
