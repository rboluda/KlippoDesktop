<!-- Una lista (o carrusel) en una sola tarjeta: nombre, miniatura, cuántos vídeos o canciones y
     acciones sobre todos a la vez. "Ver uno a uno" la despliega con la tarjeta de cada uno. -->
<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import {
    cancelarLista,
    compartirLista,
    copiarLista,
    eliminarLista,
    estaActiva,
    guardarLista,
    mostrarGuardado,
    whatsappLista,
    type Descarga,
  } from "./descargas.svelte";
  import { plataforma } from "./enlaces";
  import Icono from "./Icono.svelte";
  import { formato, t } from "./i18n";
  import TarjetaDescarga from "./TarjetaDescarga.svelte";

  let { grupo, elementos, mosaico = false }: { grupo: string; elementos: Descarga[]; mosaico?: boolean } =
    $props();

  let desplegada = $state(false);
  let menuAbierto = $state(false);

  const primero = $derived(elementos[0]);
  const conImagen = $derived(elementos.find((d) => d.miniatura) ?? primero);
  const nombre = $derived(elementos.find((d) => d.lista)?.lista ?? primero.titulo ?? "Klippo");
  const sonAudio = $derived(elementos.every((d) => d.esAudio));

  // Una lista de Spotify aparece desde el principio: lo que está terminado, lo que se baja,
  // lo que ha fallado (las canceladas no cuentan)
  const validas = $derived(elementos.filter((d) => d.estado.tipo !== "cancelada"));
  const terminadas = $derived(elementos.filter((d) => d.estado.tipo === "completada"));
  const enCurso = $derived(elementos.filter(estaActiva));
  const conError = $derived(elementos.filter((d) => d.estado.tipo === "fallida").length);
  const bajando = $derived(enCurso.length > 0);
  const avanceTotal = $derived(
    (terminadas.length + enCurso.reduce((suma, d) => suma + (d.progreso ?? 0), 0)) / Math.max(1, validas.length),
  );
  const cuantos = $derived(
    terminadas.length === validas.length
      ? formato(sonAudio ? t.canciones : t.videos, terminadas.length)
      : formato(t.descargadasDe, terminadas.length, validas.length),
  );
  const todasGuardadas = $derived(terminadas.length > 0 && terminadas.every((d) => d.guardado));
  const sinTerminadas = $derived(terminadas.length === 0);
  // Ya hay algo guardado: el Explorador se abre en esa carpeta, con ese archivo marcado
  const unaGuardada = $derived(terminadas.find((d) => d.guardado));

  function cerrarMenuFuera(evento: MouseEvent) {
    if (menuAbierto && !(evento.target as Element).closest(".menu-contenedor")) menuAbierto = false;
  }
</script>

<svelte:window onclick={cerrarMenuFuera} />

<article class="lista" class:mosaico class:desplegada>
  <div class="cabeza">
    <button class="miniatura" onclick={() => (desplegada = !desplegada)} title={desplegada ? t.ocultarLista : t.verUnoAUno}>
      {#if conImagen.miniatura}
        <img src={convertFileSrc(conImagen.miniatura)} alt="" loading="lazy" />
      {:else}
        <span class="sin-imagen"><Icono nombre={sonAudio ? "audio" : "lista"} tamano={32} /></span>
      {/if}
      <!-- Una pila: la miniatura parece una lista -->
      <span class="contador"><Icono nombre="lista" tamano={14} />{elementos.length}</span>
    </button>

    <div class="cuerpo">
      <p class="titulo" title={nombre}>{nombre}</p>
      <p class="detalle">
        {[primero.spotify ? "Spotify" : plataforma(primero.url), cuantos].join(" · ")}
        {#if conError > 0}
          · <span class="error">{formato(t.conError, conError)}</span>
        {/if}
      </p>
      {#if bajando}
        <div class="barra"><span style:width="{Math.round(avanceTotal * 100)}%"></span></div>
      {/if}
      <button class="boton texto desplegar" onclick={() => (desplegada = !desplegada)}>
        {desplegada ? t.ocultarLista : t.verUnoAUno}
      </button>
    </div>

    <div class="acciones">
      {#if bajando}
        <button class="boton texto" onclick={() => cancelarLista(grupo)}>{t.cancelar}</button>
      {/if}
      <button class="boton-icono" disabled={sinTerminadas} title={t.whatsappTodo} aria-label={t.whatsappTodo} onclick={() => whatsappLista(grupo)}>
        <Icono nombre="whatsapp" tamano={20} />
      </button>
      <button class="boton-icono" disabled={sinTerminadas} title={t.compartirTodo} aria-label={t.compartirTodo} onclick={() => compartirLista(grupo)}>
        <Icono nombre="compartir" tamano={20} />
      </button>
      <button
        class="boton-icono"
        disabled={sinTerminadas}
        title={t.copiarTodo}
        aria-label={t.copiarTodo}
        onclick={() => copiarLista(grupo, terminadas.length)}
      >
        <Icono nombre="copiar" tamano={20} />
      </button>
      <button
        class="boton-icono"
        disabled={sinTerminadas}
        class:hecho={todasGuardadas}
        title={t.guardarTodo}
        aria-label={t.guardarTodo}
        onclick={() => guardarLista(grupo, nombre)}
      >
        <Icono nombre={todasGuardadas ? "hecho" : "guardar"} tamano={20} />
      </button>
      {#if unaGuardada}
        <button
          class="boton-icono"
          title={t.verEnExplorador}
          aria-label={t.verEnExplorador}
          onclick={() => mostrarGuardado(unaGuardada.id)}
        >
          <Icono nombre="carpeta" tamano={20} />
        </button>
      {/if}
      <div class="menu-contenedor">
        <button
          class="boton-icono"
          title={t.mas}
          aria-label={t.mas}
          aria-expanded={menuAbierto}
          onclick={() => (menuAbierto = !menuAbierto)}
        >
          <Icono nombre="mas" tamano={20} />
        </button>
        {#if menuAbierto}
          <div class="menu" role="menu">
            <button
              role="menuitem"
              onclick={() => {
                menuAbierto = false;
                eliminarLista(grupo);
              }}
            >
              <Icono nombre="eliminar" tamano={20} />{t.eliminarLista}
            </button>
          </div>
        {/if}
      </div>
    </div>
  </div>

  {#if desplegada}
    <div class="elementos">
      {#each elementos as descarga (descarga.id)}
        <TarjetaDescarga {descarga} />
      {/each}
    </div>
  {/if}
</article>

<style>
  .lista {
    border-radius: 16px;
    background: var(--surface-container-low);
    /* Una sombra escalonada: se distingue de una tarjeta suelta */
    box-shadow:
      0 4px 0 -1px var(--surface-container),
      0 8px 0 -2px var(--surface-container-high);
    margin-bottom: 8px;
  }

  .cabeza {
    display: grid;
    grid-template-columns: 160px 1fr auto;
    column-gap: 16px;
    padding: 8px;
  }

  .miniatura {
    position: relative;
    aspect-ratio: 16 / 9;
    padding: 0;
    border: none;
    border-radius: 10px;
    overflow: hidden;
    background: var(--surface-container-highest);
    color: var(--on-surface-variant);
    cursor: pointer;
    align-self: start;
  }

  .miniatura img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }

  .sin-imagen {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
  }

  .contador {
    position: absolute;
    right: 6px;
    bottom: 6px;
    display: flex;
    align-items: center;
    gap: 3px;
    padding: 1px 6px;
    border-radius: 6px;
    background: rgb(0 0 0 / 0.7);
    color: #fff;
    font-size: 0.75rem;
    font-weight: 600;
  }

  .cuerpo {
    min-width: 0;
    align-self: center;
  }

  .titulo {
    margin: 0 0 4px;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    word-break: break-word;
  }

  .detalle {
    margin: 0;
    font-size: 0.87rem;
    color: var(--on-surface-variant);
  }

  .desplegar {
    height: 32px;
    margin: 4px 0 0 -12px;
  }

  .acciones {
    display: flex;
    align-items: center;
  }

  .acciones .boton-icono:disabled {
    opacity: 0.38;
    cursor: default;
  }

  .error {
    color: var(--error);
  }

  .barra {
    height: 4px;
    margin-top: 8px;
    border-radius: 2px;
    background: var(--primary-container);
    overflow: hidden;
  }

  .barra span {
    display: block;
    height: 100%;
    background: var(--primary);
    transition: width 0.3s linear;
  }

  .hecho {
    color: var(--primary);
  }

  .menu-contenedor {
    position: relative;
  }

  .menu {
    position: absolute;
    right: 0;
    top: 100%;
    z-index: 5;
    min-width: 220px;
    padding: 8px 0;
    border-radius: 4px;
    background: var(--surface-container);
    box-shadow: 0 4px 16px rgb(0 0 0 / 0.2);
  }

  .menu button {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    height: 44px;
    padding: 0 16px;
    border: none;
    background: none;
    text-align: left;
    cursor: pointer;
  }

  .menu button:hover {
    background: color-mix(in srgb, var(--on-surface) 8%, transparent);
  }

  .elementos {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 0 8px 8px 24px;
  }

  /* Mosaico: miniatura arriba; desplegada ocupa toda la fila */
  .mosaico .cabeza {
    grid-template-columns: 1fr;
    row-gap: 8px;
  }

  .mosaico .miniatura {
    width: 100%;
  }

  .mosaico .cuerpo {
    padding: 0 4px;
  }

  .mosaico .acciones {
    justify-content: flex-end;
  }

  .mosaico.desplegada {
    grid-column: 1 / -1;
  }

  .mosaico.desplegada .cabeza {
    grid-template-columns: 220px 1fr auto;
  }
</style>
