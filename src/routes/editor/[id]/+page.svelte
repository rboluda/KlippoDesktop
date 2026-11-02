<!-- Editor: el vídeo con sus capas (encuadre, textos y emojis, zonas pixeladas), la barra de
     tiempo con el recorte y las herramientas. Solo recoge parámetros: al exportar se genera
     un único comando de FFmpeg. -->
<script lang="ts">
  import { onMount } from "svelte";
  import { convertFileSrc, invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { page } from "$app/state";
  import { descargas } from "$lib/descargas.svelte";
  import CapaSuperpuesto from "$lib/edicion/CapaSuperpuesto.svelte";
  import CapaZona from "$lib/edicion/CapaZona.svelte";
  import DialogoTexto from "$lib/edicion/DialogoTexto.svelte";
  import DialogoTiempo from "$lib/edicion/DialogoTiempo.svelte";
  import LineaTiempo from "$lib/edicion/LineaTiempo.svelte";
  import {
    clave,
    ediciones,
    encuadreCentrado,
    FORMATOS,
    hayCambios,
    MINIMO_ENCUADRE,
    MINIMO_PIXELADO,
    sinCambios,
    TAMANO_EMOJI,
    TAMANO_TEXTO,
    zonaNueva,
    type EstadoExportacion,
    type EstiloTexto,
    type Formato,
    type ParametrosEdicion,
  } from "$lib/edicion/modelo";
  import PanelExportar from "$lib/edicion/PanelExportar.svelte";
  import { dibujar, png } from "$lib/edicion/render";
  import SelectorEmoji from "$lib/edicion/SelectorEmoji.svelte";
  import Icono from "$lib/Icono.svelte";
  import { t } from "$lib/i18n";

  type Herramienta = "encuadre" | "texto" | "emoji" | "pixelar";

  const id = page.params.id ?? "";
  const descarga = $derived(descargas.lista.find((d) => d.id === id));

  // Lo editado se conserva al salir y volver mientras Klippo siga abierto
  let p = $state<ParametrosEdicion>(ediciones.get(id) ?? sinCambios());
  $effect(() => {
    ediciones.set(id, $state.snapshot(p) as ParametrosEdicion);
  });

  let video = $state<HTMLVideoElement>();
  let anchoVideo = $state(0);
  let altoVideo = $state(0);
  let duracionMs = $state(0);
  let actualMs = $state(0);
  let reproduciendo = $state(false);
  let anchoZona = $state(0);
  let altoZona = $state(0);

  let herramienta = $state<Herramienta | null>(null);
  let seleccionado = $state<number | null>(null);
  let dialogo = $state<"texto" | "emoji" | "inicio" | "fin" | null>(null);
  let exportacion = $state<EstadoExportacion | null>(null);

  const aspecto = $derived(altoVideo ? anchoVideo / altoVideo : 1);
  // El vídeo ocupa lo que quepa en su zona sin deformarse; las capas van encima, a su medida
  const escala = $derived(anchoVideo && altoVideo ? Math.min(anchoZona / anchoVideo, altoZona / altoVideo) : 0);
  const anchoMarco = $derived(anchoVideo * escala);
  const altoMarco = $derived(altoVideo * escala);
  const inicioMs = $derived(p.recorte?.inicioMs ?? 0);
  const finMs = $derived(p.recorte?.finMs ?? duracionMs);

  function alCargar() {
    if (!video) return;
    anchoVideo = video.videoWidth;
    altoVideo = video.videoHeight;
    duracionMs = Math.round(video.duration * 1000);
    video.currentTime = inicioMs / 1000;
  }

  // Mientras se reproduce: la posición en la barra y, con recorte, en bucle dentro del tramo
  onMount(() => {
    let fotograma = 0;
    const seguir = () => {
      if (video) {
        actualMs = video.currentTime * 1000;
        if (reproduciendo && actualMs >= finMs) video.currentTime = inicioMs / 1000;
      }
      fotograma = requestAnimationFrame(seguir);
    };
    fotograma = requestAnimationFrame(seguir);
    return () => cancelAnimationFrame(fotograma);
  });

  function alternarReproduccion() {
    if (!video) return;
    if (video.paused) {
      if (actualMs < inicioMs || actualMs >= finMs - 50) video.currentTime = inicioMs / 1000;
      video.play();
    } else {
      video.pause();
    }
  }

  function buscar(ms: number) {
    if (video) video.currentTime = ms / 1000;
    actualMs = ms;
  }

  function cambiarRecorte(inicio: number, fin: number) {
    p.recorte = inicio <= 0 && fin >= duracionMs ? null : { inicioMs: inicio, finMs: fin };
  }

  function elegirFormato(formato: Formato | null) {
    p.encuadre = formato ? encuadreCentrado(formato, aspecto) : null;
  }

  function anadirSuperpuesto(texto: string, estilo: EstiloTexto | null) {
    const nuevo = {
      id: Date.now(),
      texto,
      x: 0.5,
      y: 0.5,
      tamano: estilo ? TAMANO_TEXTO : TAMANO_EMOJI,
      estilo,
    };
    p.superpuestos.push(nuevo);
    seleccionado = nuevo.id;
    dialogo = null;
  }

  function anadirZona() {
    const zona = zonaNueva(aspecto);
    p.pixelados.push(zona);
    seleccionado = zona.id;
  }

  function quitar(idElemento: number) {
    p.superpuestos = p.superpuestos.filter((s) => s.id !== idElemento);
    p.pixelados = p.pixelados.filter((z) => z.id !== idElemento);
    if (seleccionado === idElemento) seleccionado = null;
  }

  function alPulsarTecla(evento: KeyboardEvent) {
    if (dialogo || exportacion || evento.target instanceof HTMLInputElement) return;
    if (evento.key === " ") {
      evento.preventDefault();
      alternarReproduccion();
    } else if ((evento.key === "Delete" || evento.key === "Backspace") && seleccionado !== null) {
      quitar(seleccionado);
    }
  }

  async function exportar() {
    if (!descarga?.archivo) return;
    video?.pause();
    if (!hayCambios(p)) {
      // Sin cambios no hace falta FFmpeg: se comparte el original
      exportacion = { fase: "listo", ruta: descarga.archivo };
      return;
    }
    exportacion = { fase: "exportando", progreso: 0 };
    const dejarDeEscuchar = await listen<number>("exportacion", (e) => {
      if (exportacion?.fase === "exportando") exportacion = { fase: "exportando", progreso: e.payload };
    });
    try {
      const superpuestos = p.superpuestos.map((s) => {
        const d = dibujar(s, anchoVideo, altoVideo);
        return { png: png(d), x: d.x, y: d.y };
      });
      const ruta = await invoke<string>("exportar", {
        id,
        parametros: {
          recorte: p.recorte,
          encuadre: p.encuadre,
          superpuestos,
          pixelados: p.pixelados,
          silenciar: p.silenciar,
          ancho: anchoVideo,
          alto: altoVideo,
          duracionMs,
          clave: clave(p),
        },
      });
      exportacion = { fase: "listo", ruta };
    } catch (e) {
      exportacion = String(e) === "cancelada" ? null : { fase: "error", mensaje: String(e) };
    } finally {
      dejarDeEscuchar();
    }
  }

  const ayuda = $derived(
    herramienta === "encuadre" && p.encuadre
      ? t.ayudaEncuadre
      : herramienta === "pixelar"
        ? t.ayudaPixelar
        : herramienta === "texto" || herramienta === "emoji"
          ? t.ayudaSuperpuesto
          : "",
  );
</script>

<svelte:window onkeydown={alPulsarTecla} />

<div class="editor">
  <div class="cabecera">
    <a class="boton-icono" href="/" title={t.atras} aria-label={t.atras}><Icono nombre="atras" /></a>
    <h1 title={descarga?.titulo ?? ""}>{descarga?.titulo ?? t.editor}</h1>
    <button class="boton" onclick={exportar} disabled={!descarga?.archivo || !duracionMs}>{t.exportar}</button>
  </div>

  {#if !descarga?.archivo}
    <p class="no-disponible">{t.noDisponible}</p>
  {:else}
    <div class="zona-video" bind:clientWidth={anchoZona} bind:clientHeight={altoZona}>
      <div
        class="marco"
        style:width="{anchoMarco}px"
        style:height="{altoMarco}px"
        onpointerdown={() => (seleccionado = null)}
        role="presentation"
      >
        <!-- svelte-ignore a11y_media_has_caption -->
        <video
          bind:this={video}
          src={convertFileSrc(descarga.archivo)}
          muted={p.silenciar}
          onloadedmetadata={alCargar}
          onplay={() => (reproduciendo = true)}
          onpause={() => (reproduciendo = false)}
          onclick={alternarReproduccion}
          preload="auto"
        ></video>

        {#if escala > 0}
          {#each p.pixelados as zona, i (zona.id)}
            <CapaZona
              zona={zona}
              anchoPx={anchoMarco}
              altoPx={altoMarco}
              {aspecto}
              minimo={MINIMO_PIXELADO}
              tipo="pixelado"
              activa={herramienta === "pixelar"}
              onchange={(z) => (p.pixelados[i] = { ...z, id: zona.id })}
              onquitar={() => quitar(zona.id)}
            />
          {/each}
          {#each p.superpuestos as elemento, i (elemento.id)}
            <CapaSuperpuesto
              {elemento}
              anchoPx={anchoMarco}
              altoPx={altoMarco}
              {anchoVideo}
              {altoVideo}
              seleccionado={seleccionado === elemento.id}
              activo={herramienta !== "encuadre" && herramienta !== "pixelar"}
              onseleccionar={() => (seleccionado = elemento.id)}
              onchange={(s) => (p.superpuestos[i] = s)}
              onquitar={() => quitar(elemento.id)}
            />
          {/each}
          {#if p.encuadre}
            {@const encuadre = p.encuadre}
            <CapaZona
              zona={encuadre}
              anchoPx={anchoMarco}
              altoPx={altoMarco}
              {aspecto}
              proporcion={FORMATOS[encuadre.formato].proporcion}
              minimo={MINIMO_ENCUADRE}
              tipo="encuadre"
              activa={herramienta === "encuadre"}
              onchange={(z) => (p.encuadre = { ...z, formato: encuadre.formato })}
            />
          {/if}
        {/if}
      </div>
    </div>

    <div class="controles">
      <button
        class="boton-icono"
        onclick={alternarReproduccion}
        title={t.reproducirPausar}
        aria-label={t.reproducirPausar}
      >
        <Icono nombre={reproduciendo ? "pausa" : "reproducir"} />
      </button>
      <div class="linea">
        <LineaTiempo
          {duracionMs}
          {actualMs}
          {inicioMs}
          {finMs}
          onbuscar={buscar}
          onrecorte={cambiarRecorte}
          oneditar={(cual) => (dialogo = cual)}
        />
      </div>
    </div>

    {#if herramienta}
      <div class="panel">
        {#if herramienta === "encuadre"}
          <div class="chips">
            <button class="chip" class:activo={!p.encuadre} onclick={() => elegirFormato(null)}>{t.original}</button>
            {#each Object.entries(FORMATOS) as [formato, datos] (formato)}
              <button
                class="chip"
                class:activo={p.encuadre?.formato === formato}
                onclick={() => elegirFormato(formato as Formato)}
              >
                {datos.etiqueta || t.libre}
              </button>
            {/each}
          </div>
        {:else if herramienta === "texto"}
          <button class="boton tonal" onclick={() => (dialogo = "texto")}>{t.anadirTexto}</button>
        {:else if herramienta === "emoji"}
          <button class="boton tonal" onclick={() => (dialogo = "emoji")}>{t.anadirEmoji}</button>
        {:else if herramienta === "pixelar"}
          <button class="boton tonal" onclick={anadirZona}>{t.anadirZona}</button>
        {/if}
        <span class="ayuda">{ayuda}</span>
      </div>
    {/if}

    <nav class="herramientas">
      {#each [["encuadre", t.encuadre], ["texto", t.texto], ["emoji", t.emoji], ["pixelar", t.pixelar]] as [clave_, nombre] (clave_)}
        <button
          class="herramienta"
          class:activa={herramienta === clave_}
          onclick={() => (herramienta = herramienta === clave_ ? null : (clave_ as Herramienta))}
        >
          <Icono nombre={clave_ as Herramienta} />
          <span>{nombre}</span>
        </button>
      {/each}
      <button class="herramienta" class:activa={p.silenciar} onclick={() => (p.silenciar = !p.silenciar)}>
        <Icono nombre={p.silenciar ? "silencio" : "volumen"} />
        <span>{p.silenciar ? t.activarSonido : t.silenciar}</span>
      </button>
    </nav>
  {/if}
</div>

{#if dialogo === "texto"}
  <DialogoTexto onaceptar={(texto, estilo) => anadirSuperpuesto(texto, estilo)} oncerrar={() => (dialogo = null)} />
{:else if dialogo === "emoji"}
  <SelectorEmoji onelegir={(emoji) => anadirSuperpuesto(emoji, null)} oncerrar={() => (dialogo = null)} />
{:else if dialogo === "inicio"}
  <DialogoTiempo
    cual="inicio"
    valorMs={inicioMs}
    minimoMs={0}
    maximoMs={finMs - 100}
    onaceptar={(ms) => (cambiarRecorte(ms, finMs), buscar(ms))}
    oncerrar={() => (dialogo = null)}
  />
{:else if dialogo === "fin"}
  <DialogoTiempo
    cual="fin"
    valorMs={finMs}
    minimoMs={inicioMs + 100}
    maximoMs={duracionMs}
    onaceptar={(ms) => (cambiarRecorte(inicioMs, ms), buscar(ms))}
    oncerrar={() => (dialogo = null)}
  />
{/if}

{#if exportacion}
  <PanelExportar
    estado={exportacion}
    titulo={descarga?.titulo ?? "Klippo"}
    oncancelar={() => invoke("cancelar_exportacion")}
    oncerrar={() => (exportacion = null)}
  />
{/if}

<style>
  .editor {
    display: flex;
    flex-direction: column;
    gap: 10px;
    height: calc(100vh - 96px);
  }

  .cabecera {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-left: -8px;
  }

  h1 {
    flex: 1;
    margin: 0;
    font-size: 1.2rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .no-disponible {
    margin: 20vh auto;
    color: var(--on-surface-variant);
  }

  .zona-video {
    flex: 1;
    min-height: 0;
    display: grid;
    place-items: center;
    border-radius: 16px;
    background: #000;
    overflow: hidden;
  }

  /* Sin recortar: los tiradores de las esquinas pueden asomar por fuera del vídeo */
  .marco {
    position: relative;
  }

  video {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    display: block;
  }

  .controles {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .linea {
    flex: 1;
  }

  .panel {
    display: flex;
    align-items: center;
    gap: 16px;
    min-height: 44px;
  }

  .ayuda {
    font-size: 0.87rem;
    color: var(--on-surface-variant);
  }

  .chips {
    display: flex;
    gap: 8px;
  }

  .chip {
    height: 32px;
    padding: 0 14px;
    border: 1px solid var(--outline);
    border-radius: 8px;
    background: transparent;
    cursor: pointer;
  }

  .chip.activo {
    border-color: transparent;
    background: var(--secondary-container);
    color: var(--on-secondary-container);
    font-weight: 600;
  }

  .herramientas {
    display: flex;
    justify-content: center;
    gap: 4px;
    padding: 6px;
    border-radius: 20px;
    background: var(--surface-container);
  }

  .herramienta {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
    min-width: 96px;
    padding: 6px 12px;
    border: none;
    border-radius: 14px;
    background: transparent;
    color: var(--on-surface-variant);
    font-size: 0.8rem;
    cursor: pointer;
  }

  .herramienta:hover {
    background: color-mix(in srgb, var(--on-surface) 6%, transparent);
  }

  .herramienta.activa {
    background: var(--secondary-container);
    color: var(--on-secondary-container);
    font-weight: 600;
  }
</style>
