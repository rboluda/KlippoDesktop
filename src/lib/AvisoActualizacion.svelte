<!-- Franja de inicio con la versión nueva de Klippo: Actualizar y listo -->
<script lang="ts">
  import { actualizacion, instalar } from "./actualizacion.svelte";
  import Icono from "./Icono.svelte";
  import { formato, t } from "./i18n";

  let descartada = $state(false);
</script>

{#if actualizacion.disponible && !descartada}
  <div class="aviso" role="status">
    <span class="texto">
      {#if actualizacion.progreso !== null}
        {formato(t.descargandoActualizacion, actualizacion.progreso)}
      {:else if actualizacion.error}
        {formato(t.errorActualizar, actualizacion.error)}
      {:else}
        {formato(t.disponible, actualizacion.disponible.version)}
      {/if}
    </span>
    {#if actualizacion.progreso === null}
      <button class="boton" onclick={instalar}>{t.actualizar}</button>
      <button class="boton-icono" title={t.cerrar} aria-label={t.cerrar} onclick={() => (descartada = true)}>
        <Icono nombre="cerrar" tamano={20} />
      </button>
    {:else}
      <progress max="100" value={actualizacion.progreso}></progress>
    {/if}
  </div>
{/if}

<style>
  .aviso {
    display: flex;
    align-items: center;
    gap: 12px;
    margin: 0 20px 8px;
    padding: 8px 8px 8px 20px;
    border-radius: 16px;
    background: var(--tertiary-container);
    color: var(--on-tertiary-container);
  }

  .texto {
    flex: 1;
    font-weight: 600;
  }

  .aviso .boton-icono {
    color: inherit;
  }

  progress {
    width: 160px;
    accent-color: var(--primary);
  }
</style>
