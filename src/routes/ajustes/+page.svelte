<!-- Ajustes: descargas, motor, almacenamiento, actualizaciones y Acerca de -->
<script lang="ts">
  import { onMount } from "svelte";
  import { getVersion } from "@tauri-apps/api/app";
  import { invoke } from "@tauri-apps/api/core";
  import * as autoarranque from "@tauri-apps/plugin-autostart";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { actualizacion, buscarAhora } from "$lib/actualizacion.svelte";
  import { guardar, leer } from "$lib/ajustes";
  import { CALIDADES, descargas, type Calidad } from "$lib/descargas.svelte";
  import Icono from "$lib/Icono.svelte";
  import { formatearTamano, formato, nombreCalidad, t } from "$lib/i18n";
  import { actualizarMotor, cargarVersiones, motor } from "$lib/motor.svelte";
  import { cargarSesiones, cerrarSesion, iniciarSesion, sesiones } from "$lib/sesiones.svelte";
  import { aplicarTema, type Tema } from "$lib/tema";

  // Componentes libres en los que se apoya Klippo, con su licencia
  const COMPONENTES = [
    { nombre: "yt-dlp", licencia: "Unlicense", url: "https://github.com/yt-dlp/yt-dlp" },
    { nombre: "FFmpeg (gyan.dev)", licencia: "GPL-3.0", url: "https://www.gyan.dev/ffmpeg/builds/" },
    { nombre: "Deno", licencia: "MIT", url: "https://deno.com" },
    { nombre: "Tauri", licencia: "MIT / Apache 2.0", url: "https://tauri.app" },
    { nombre: "Svelte", licencia: "MIT", url: "https://svelte.dev" },
  ];

  const REPO = "https://github.com/rboluda/KlippoDesktop";

  let version = $state("");
  let buscarAuto = $state(true);
  let buscando = $state(false);
  let resultado = $state("");
  let preguntarCalidad = $state(true);
  let calidad = $state<Calidad>("mejor");
  let vista = $state<"lista" | "mosaico">("lista");
  let tamanoCache = $state(0);
  let tema = $state<Tema>("sistema");
  let vigilarPortapapeles = $state(true);
  let conWindows = $state(false);
  let avisoMarcador = $state("");

  onMount(async () => {
    version = await getVersion();
    buscarAuto = await leer("buscarActualizaciones", true);
    preguntarCalidad = await leer("preguntarCalidad", true);
    calidad = await leer<Calidad>("calidad", "mejor");
    vista = await leer("vista", "lista");
    vigilarPortapapeles = await leer("vigilarPortapapeles", true);
    tema = await leer<Tema>("tema", "sistema");
    conWindows = await autoarranque.isEnabled().catch(() => false);
    cargarVersiones();
    cargarSesiones();
  });

  function cambiarVigilarPortapapeles() {
    vigilarPortapapeles = !vigilarPortapapeles;
    guardar("vigilarPortapapeles", vigilarPortapapeles);
  }

  async function cambiarConWindows() {
    try {
      await (conWindows ? autoarranque.disable() : autoarranque.enable());
    } finally {
      conWindows = await autoarranque.isEnabled().catch(() => false);
    }
  }

  async function copiarCodigoMarcador() {
    await navigator.clipboard.writeText(await invoke<string>("codigo_marcador"));
    avisoMarcador = t.codigoCopiado;
  }

  // El tamaño de la caché se vuelve a medir cada vez que cambia la lista de descargas
  $effect(() => {
    void descargas.lista;
    invoke<number>("tamano_cache").then((t) => (tamanoCache = t));
  });

  function cambiarPreguntarCalidad() {
    preguntarCalidad = !preguntarCalidad;
    guardar("preguntarCalidad", preguntarCalidad);
  }

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
    <h2>{t.apariencia}</h2>
    <div class="tarjeta">
      <label class="fila">
        <span class="textos"><span class="titulo">{t.tema}</span></span>
        <select
          bind:value={tema}
          onchange={() => {
            guardar("tema", tema);
            aplicarTema(tema);
          }}
        >
          <option value="sistema">{t.temaSistema}</option>
          <option value="claro">{t.temaClaro}</option>
          <option value="oscuro">{t.temaOscuro}</option>
        </select>
      </label>
    </div>
  </section>

  <section>
    <h2>{t.descargas}</h2>
    <div class="tarjeta">
      <button class="fila" role="switch" aria-checked={preguntarCalidad} onclick={cambiarPreguntarCalidad}>
        <span class="textos">
          <span class="titulo">{t.preguntarCalidad}</span>
          <span class="detalle">{t.preguntarCalidadDetalle}</span>
        </span>
        <span class="interruptor" class:activo={preguntarCalidad}><span></span></span>
      </button>
      <label class="fila">
        <span class="textos"><span class="titulo">{t.calidadMarcada}</span></span>
        <select bind:value={calidad} onchange={() => guardar("calidad", calidad)}>
          {#each CALIDADES as c (c)}
            <option value={c}>{nombreCalidad(c)}</option>
          {/each}
        </select>
      </label>
      <label class="fila">
        <span class="textos"><span class="titulo">{t.vista}</span></span>
        <select bind:value={vista} onchange={() => guardar("vista", vista)}>
          <option value="lista">{t.vistaLista}</option>
          <option value="mosaico">{t.vistaMosaico}</option>
        </select>
      </label>
    </div>
  </section>

  <section>
    <h2>{t.navegador}</h2>
    <div class="tarjeta">
      <button class="fila" role="switch" aria-checked={vigilarPortapapeles} onclick={cambiarVigilarPortapapeles}>
        <span class="textos">
          <span class="titulo">{t.vigilarPortapapeles}</span>
          <span class="detalle">{t.vigilarPortapapelesDetalle}</span>
        </span>
        <span class="interruptor" class:activo={vigilarPortapapeles}><span></span></span>
      </button>
      <button class="fila" role="switch" aria-checked={conWindows} onclick={cambiarConWindows}>
        <span class="textos">
          <span class="titulo">{t.iniciarConWindows}</span>
          <span class="detalle">{t.iniciarConWindowsDetalle}</span>
        </span>
        <span class="interruptor" class:activo={conWindows}><span></span></span>
      </button>
      <div class="fila">
        <span class="textos">
          <span class="titulo">{t.marcador}</span>
          <span class="detalle">{avisoMarcador || t.marcadorDetalle}</span>
        </span>
        <button class="boton texto" onclick={copiarCodigoMarcador}>{t.copiarCodigo}</button>
        <button class="boton tonal" onclick={() => invoke("instalar_marcador")}>{t.instalar}</button>
      </div>
    </div>
  </section>

  <section>
    <h2>{t.cuentas}</h2>
    <div class="tarjeta">
      {#each sesiones.lista as sesion (sesion.plataforma)}
        <div class="fila">
          <span class="textos">
            <span class="titulo">{sesion.nombre}</span>
            <span class="detalle">{sesion.iniciada ? t.sesionIniciada : t.sinSesion}</span>
          </span>
          {#if sesion.iniciada}
            <button class="boton texto" onclick={() => cerrarSesion(sesion.plataforma)}>{t.cerrarSesion}</button>
          {:else}
            <button class="boton tonal" onclick={() => iniciarSesion(sesion.plataforma)}>{t.iniciarSesion}</button>
          {/if}
        </div>
      {/each}
    </div>
    <p class="nota">{t.cuentasNota}</p>
  </section>

  <section>
    <h2>{t.motor}</h2>
    <div class="tarjeta">
      <div class="fila">
        <span class="textos">
          <span class="titulo">{t.motorDetalle}</span>
          {#if motor.versiones}
            <span class="detalle">
              {formato(t.motorVersiones, motor.versiones["yt-dlp"], motor.versiones.ffmpeg, motor.versiones.deno)}
            </span>
          {/if}
          {#if motor.mensaje}<span class="detalle">{motor.mensaje}</span>{/if}
        </span>
        <button class="boton tonal" onclick={actualizarMotor} disabled={motor.actualizando}>
          {motor.actualizando ? t.actualizandoMotor : t.actualizar}
        </button>
      </div>
    </div>
  </section>

  <section>
    <h2>{t.almacenamiento}</h2>
    <div class="tarjeta">
      <div class="fila">
        <span class="textos">
          <span class="titulo">{t.cache}</span>
          <span class="detalle">{formato(t.cacheDetalle, formatearTamano(tamanoCache))}</span>
        </span>
        <button class="boton tonal" onclick={() => invoke("vaciar_cache")} disabled={tamanoCache === 0}>
          {t.vaciar}
        </button>
      </div>
    </div>
  </section>

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
        <p class="detalle subtitulo">{t.softwareLibre}</p>
        <ul class="componentes">
          {#each COMPONENTES as c (c.nombre)}
            <li>
              <button class="enlace" onclick={() => openUrl(c.url)}>{c.nombre}</button>
              <span class="detalle">{c.licencia}</span>
            </li>
          {/each}
        </ul>
        {#if motor.versiones}
          <button
            class="enlace"
            onclick={() => openUrl(`https://ffmpeg.org/releases/ffmpeg-${motor.versiones!.ffmpeg}.tar.xz`)}
          >
            {formato(t.fuentesFfmpeg, motor.versiones.ffmpeg)}
          </button>
        {/if}
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

  button.fila,
  label.fila {
    cursor: pointer;
  }

  .subtitulo {
    margin-top: 16px;
    font-weight: 600;
  }

  .componentes {
    margin: 4px 0 8px;
    padding: 0;
    list-style: none;
  }

  .componentes li {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    padding: 3px 0;
  }

  .enlace {
    padding: 0;
    border: none;
    background: none;
    color: var(--primary);
    text-decoration: underline;
    text-underline-offset: 2px;
    cursor: pointer;
  }

  .nota {
    margin: 8px 4px 0;
    font-size: 0.85rem;
    color: var(--on-surface-variant);
  }

  select {
    height: 40px;
    padding: 0 12px;
    border: 1px solid var(--outline);
    border-radius: 8px;
    background: var(--surface-container-lowest);
    color: var(--on-surface);
    font: inherit;
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
