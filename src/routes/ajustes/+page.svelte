<!-- Ajustes: actualizaciones y Acerca de -->
<script lang="ts">
  import { onMount } from "svelte";
  import { getVersion } from "@tauri-apps/api/app";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { actualizacion, buscarAhora } from "$lib/actualizacion.svelte";
  import { guardar, leer } from "$lib/ajustes";
  import Icono from "$lib/Icono.svelte";
  import { formato, t } from "$lib/i18n";

  const REPO = "https://github.com/rboluda/KlippoDesktop";

  let version = $state("");
  let buscarAuto = $state(true);
  let buscando = $state(false);
  let resultado = $state("");

  onMount(async () => {
    version = await getVersion();
    buscarAuto = await leer("buscarActualizaciones", true);
  });

  function cambiarBuscarAuto() {
    buscarAuto = !buscarAuto;
    guardar("buscarActualizaciones", buscarAuto);
  }

  async function buscar() {
    buscando = true;
    resultado = "";
    try {
      const hayNueva = await buscarAhora();
      resultado = hayNueva ? formato(t.disponible, actualizacion.disponible!.version) : t.alDia;
    } catch {
      resultado = t.errorComprobar;
    } finally {
      buscando = false;
    }
  }
</script>

<div class="cabecera">
  <a class="boton-icono" href="/" title={t.atras} aria-label={t.atras}><Icono nombre="atras" /></a>
  <h1>{t.ajustes}</h1>
</div>

<div class="columna">
  <section>
    <h2>{t.actualizaciones}</h2>
    <div class="tarjeta">
      <button class="fila" role="switch" aria-checked={buscarAuto} onclick={cambiarBuscarAuto}>
        <span class="textos">
          <span class="titulo">{t.buscarAuto}</span>
          <span class="detalle">{t.buscarAutoDetalle}</span>
        </span>
        <span class="interruptor" class:activo={buscarAuto}><span></span></span>
      </button>
      <div class="fila">
        <span class="textos">
          <span class="titulo">{t.buscarAhora}</span>
          {#if resultado}<span class="detalle">{resultado}</span>{/if}
        </span>
        <button class="boton tonal" onclick={buscar} disabled={buscando}>
          {buscando ? t.buscando : t.buscar}
        </button>
      </div>
    </div>
  </section>

  <section>
    <h2>{t.acercaDe}</h2>
    <div class="tarjeta acerca">
      <img src="/klippo.png" alt="" width="64" height="64" />
      <div>
        <p class="nombre">Klippo <span class="version">v{version}</span></p>
        <p>{t.descripcion}</p>
        <p class="detalle">{t.principios}</p>
        <p class="detalle">{t.autor} · {t.licencia}</p>
        <div class="enlaces">
          <button class="boton texto" onclick={() => openUrl(REPO)}>{t.codigo}</button>
          <button class="boton texto" onclick={() => openUrl("https://github.com/rboluda")}>{t.miGithub}</button>
          <button class="boton texto" onclick={() => openUrl(`${REPO}/blob/main/LICENSE`)}>GPL-3.0</button>
        </div>
      </div>
    </div>
  </section>
</div>

<style>
  .cabecera {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0 0 8px -8px;
  }

  h1 {
    margin: 0;
    font-size: 1.4rem;
  }

  .columna {
    max-width: 680px;
  }

  h2 {
    margin: 20px 0 8px 4px;
    font-size: 0.9rem;
    font-weight: 600;
    color: var(--primary);
  }

  .tarjeta {
    border-radius: 16px;
    background: var(--surface-container-low);
    overflow: hidden;
  }

  .fila {
    display: flex;
    align-items: center;
    gap: 16px;
    width: 100%;
    min-height: 64px;
    padding: 10px 16px;
    border: none;
    background: transparent;
    text-align: left;
  }

  .fila + .fila {
    border-top: 1px solid var(--outline-variant);
  }

  button.fila {
    cursor: pointer;
  }

  button.fila:hover {
    background: color-mix(in srgb, var(--on-surface) 5%, transparent);
  }

  .textos {
    flex: 1;
    display: flex;
    flex-direction: column;
  }

  .titulo {
    font-weight: 500;
  }

  .detalle {
    font-size: 0.87rem;
    color: var(--on-surface-variant);
  }

  /* Interruptor de Material 3 */
  .interruptor {
    position: relative;
    flex: none;
    width: 52px;
    height: 32px;
    border-radius: 16px;
    border: 2px solid var(--outline);
    background: var(--surface-container-highest);
    transition: background 0.15s;
  }

  .interruptor span {
    position: absolute;
    top: 6px;
    left: 6px;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: var(--outline);
    transition: all 0.15s;
  }

  .interruptor.activo {
    border-color: var(--primary);
    background: var(--primary);
  }

  .interruptor.activo span {
    top: 2px;
    left: 22px;
    width: 24px;
    height: 24px;
    background: var(--on-primary);
  }

  .acerca {
    display: flex;
    gap: 20px;
    padding: 20px;
  }

  .acerca img {
    border-radius: 14px;
  }

  .acerca p {
    margin: 0 0 8px;
  }

  .nombre {
    font-size: 1.15rem;
    font-weight: 700;
  }

  .version {
    font-weight: 400;
    font-size: 0.9rem;
    color: var(--on-surface-variant);
  }

  .enlaces {
    display: flex;
    flex-wrap: wrap;
    margin-left: -12px;
  }
</style>
