<!-- Exportar: progreso mientras FFmpeg genera el vídeo y, al terminar, qué hacer con él -->
<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import Icono from "../Icono.svelte";
  import { formato, t } from "../i18n";
  import { mostrarMensaje } from "../mensajes.svelte";
  import type { EstadoExportacion } from "./modelo";

  let {
    estado,
    titulo,
    oncancelar,
    oncerrar,
  }: { estado: EstadoExportacion; titulo: string; oncancelar: () => void; oncerrar: () => void } = $props();

  let guardado = $state<string | null>(null);

  async function intentar(accion: () => Promise<unknown>) {
    try {
      await accion();
    } catch (e) {
      mostrarMensaje({ texto: formato(t.noSePudo, String(e)) });
    }
  }

  const ruta = $derived(estado.fase === "listo" ? estado.ruta : "");

  const acciones = {
    whatsapp: () =>
      intentar(async () => {
        await invoke("whatsapp_archivo", { ruta });
        mostrarMensaje({ texto: t.whatsappAbierto });
      }),
    compartir: () => intentar(() => invoke("compartir_archivo", { ruta, titulo })),
    copiar: () =>
      intentar(async () => {
        await invoke("copiar_archivo", { ruta });
        mostrarMensaje({ texto: t.copiado });
      }),
    guardar: () =>
      intentar(async () => {
        if (!guardado) guardado = await invoke<string>("guardar_archivo", { ruta });
        const enDescargas = guardado;
        mostrarMensaje({
          texto: t.guardado,
          accion: { texto: t.mostrar, hacer: () => invoke("mostrar_archivo", { ruta: enDescargas }) },
        });
      }),
    reproducir: () => intentar(() => invoke("reproducir_archivo", { ruta })),
  };
</script>

<div class="fondo">
  <div class="panel" role="dialog" aria-modal="true">
    {#if estado.fase === "exportando"}
      <h2>{formato(t.exportando, Math.round(estado.progreso * 100))}</h2>
      <progress max="1" value={estado.progreso}></progress>
      <div class="pie">
        <button class="boton texto" onclick={oncancelar}>{t.cancelar}</button>
      </div>
    {:else if estado.fase === "error"}
      <h2>{t.exportarError}</h2>
      <p class="error">{estado.mensaje}</p>
      <div class="pie">
        <button class="boton" onclick={oncerrar}>{t.cerrar}</button>
      </div>
    {:else}
      <h2>{t.videoListo}</h2>
      <p>{t.videoListoDetalle}</p>
      <div class="acciones">
        <button class="accion" onclick={acciones.whatsapp}><Icono nombre="whatsapp" />WhatsApp</button>
        <button class="accion" onclick={acciones.compartir}><Icono nombre="compartir" />{t.compartir}</button>
        <button class="accion" onclick={acciones.copiar}><Icono nombre="copiar" />{t.copiarCorto}</button>
        <button class="accion" onclick={acciones.guardar}>
          <Icono nombre={guardado ? "hecho" : "guardar"} />{guardado ? t.guardadoCorto : t.guardarCorto}
        </button>
        <button class="accion" onclick={acciones.reproducir}><Icono nombre="reproducir" />{t.reproducir}</button>
      </div>
      <div class="pie">
        <button class="boton texto" onclick={oncerrar}>{t.seguirEditando}</button>
      </div>
    {/if}
  </div>
</div>

<style>
  .fondo {
    position: fixed;
    inset: 0;
    z-index: 15;
    display: grid;
    place-items: center;
    background: rgb(0 0 0 / 0.4);
  }

  .panel {
    width: min(520px, calc(100vw - 32px));
    padding: 24px;
    border-radius: 28px;
    background: var(--surface-container-high);
    color: var(--on-surface);
  }

  h2 {
    margin: 0 0 12px;
    font-size: 1.3rem;
    font-weight: 500;
  }

  p {
    margin: 0 0 16px;
    color: var(--on-surface-variant);
  }

  .error {
    color: var(--error);
    user-select: text;
    word-break: break-word;
  }

  progress {
    width: 100%;
    height: 6px;
    accent-color: var(--primary);
  }

  .acciones {
    display: grid;
    grid-template-columns: repeat(5, 1fr);
    gap: 8px;
  }

  .accion {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    padding: 12px 4px;
    border: none;
    border-radius: 16px;
    background: var(--secondary-container);
    color: var(--on-secondary-container);
    font-size: 0.85rem;
    cursor: pointer;
  }

  .accion:hover {
    filter: brightness(0.97);
  }

  .pie {
    display: flex;
    justify-content: flex-end;
    margin-top: 16px;
  }
</style>
