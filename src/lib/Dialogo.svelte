<!-- Diálogo modal al estilo Material 3: título, contenido y Cancelar / Aceptar -->
<script lang="ts">
  import { onMount, type Snippet } from "svelte";
  import { t } from "./i18n";

  let {
    titulo,
    aceptar = t.aceptar,
    puedeAceptar = true,
    onaceptar,
    oncerrar,
    children,
  }: {
    titulo: string;
    aceptar?: string;
    puedeAceptar?: boolean;
    /** Devuelve false para no cerrar (p. ej. si hay un error que enseñar) */
    onaceptar: () => boolean | void;
    oncerrar: () => void;
    children: Snippet;
  } = $props();

  let dialogo: HTMLDialogElement;

  onMount(() => dialogo.showModal());

  function enviar(evento: SubmitEvent) {
    evento.preventDefault();
    if (onaceptar() !== false) dialogo.close();
  }
</script>

<dialog bind:this={dialogo} onclose={oncerrar}>
  <form onsubmit={enviar}>
    <h2>{titulo}</h2>
    {@render children()}
    <div class="botones">
      <button type="button" class="boton texto" onclick={() => dialogo.close()}>{t.cancelar}</button>
      <button type="submit" class="boton" disabled={!puedeAceptar}>{aceptar}</button>
    </div>
  </form>
</dialog>

<style>
  dialog {
    width: min(420px, calc(100vw - 32px));
    padding: 24px;
    border: none;
    border-radius: 28px;
    background: var(--surface-container-high);
    color: var(--on-surface);
  }

  dialog::backdrop {
    background: rgb(0 0 0 / 0.32);
  }

  h2 {
    margin: 0 0 16px;
    font-size: 1.3rem;
    font-weight: 500;
  }

  .botones {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 20px;
  }

  dialog :global(input[type="text"]) {
    width: 100%;
    height: 48px;
    padding: 0 14px;
    border: 1px solid var(--outline);
    border-radius: 8px;
    background: var(--surface-container-lowest);
    color: var(--on-surface);
    font: inherit;
    user-select: text;
  }

  dialog :global(input[type="text"]:focus) {
    outline: 2px solid var(--primary);
    border-color: transparent;
  }
</style>
