<script lang="ts">
  import { cerrarMensaje, mensajes } from "./mensajes.svelte";
</script>

{#if mensajes.actual}
  {@const mensaje = mensajes.actual}
  <div class="mensaje" role="status">
    <span>{mensaje.texto}</span>
    {#if mensaje.accion}
      <button
        class="boton texto"
        onclick={() => {
          mensaje.accion?.hacer();
          cerrarMensaje();
        }}
      >
        {mensaje.accion.texto}
      </button>
    {/if}
  </div>
{/if}

<style>
  .mensaje {
    position: fixed;
    left: 50%;
    bottom: 24px;
    z-index: 20;
    transform: translateX(-50%);
    display: flex;
    align-items: center;
    gap: 12px;
    max-width: min(640px, calc(100vw - 32px));
    min-height: 48px;
    padding: 6px 8px 6px 16px;
    border-radius: 8px;
    background: var(--on-surface);
    color: var(--surface);
    box-shadow: 0 4px 16px rgb(0 0 0 / 0.25);
  }

  .mensaje .boton.texto {
    color: var(--primary-container);
    flex: none;
  }
</style>
