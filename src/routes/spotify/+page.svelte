<!-- Descargar desde Spotify: las canciones de la lista (o del álbum, o la canción), lo que se ha
     encontrado de cada una en YouTube y lo parecido que es, y su descarga -->
<script lang="ts">
  import { onMount } from "svelte";
  import { convertFileSrc, invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { page } from "$app/state";
  import { guardar, leer } from "$lib/ajustes";
  import { descargas, estaActiva, formatearDuracion, mostrarGuardado, reintentar, type Calidad, type Descarga } from "$lib/descargas.svelte";
  import Icono from "$lib/Icono.svelte";
  import { formato, t } from "$lib/i18n";
  import { mostrarMensaje } from "$lib/mensajes.svelte";
  import { gruposSpotify } from "$lib/spotify";

  interface Cancion {
    posicion: number;
    titulo: string;
    artistas: string[];
    duracionMs: number;
    uri: string;
  }
  interface Coleccion {
    tipo: "playlist" | "album" | "track";
    id: string;
    nombre: string;
    autor: string | null;
    portada: string | null;
    canciones: Cancion[];
    totalReal: number | null;
  }
  interface Resultado {
    id: string;
    titulo: string;
    duracion: number | null;
    canal: string | null;
  }
  interface Coincidencia {
    grado: "identica" | "parecida" | "noEncontrada";
    elegido: Resultado | null;
    puntos: number;
    alternativas: Resultado[];
  }

  const enlace = page.url.searchParams.get("enlace") ?? "";
  const sesion = Date.now();

  let coleccion = $state<Coleccion | null>(null);
  let error = $state("");
  let coincidencias = $state<Record<number, Coincidencia>>({});
  let buscando = $state(true);
  let calidad = $state<Calidad>("audio");
  let guardarAlTerminar = $state(true);

  const recibidas = $derived(Object.keys(coincidencias).length);
  const total = $derived(coleccion?.canciones.length ?? 0);
  const cuenta = (g: Coincidencia["grado"]) => Object.values(coincidencias).filter((c) => c.grado === g).length;
  // Con la misma lista, el mismo grupo: lo que se pida después se junta con lo de antes
  let grupo = $state<string | null>(null);

  /** La descarga de una canción (la más reciente de esta lista) */
  function descargaDe(c: Cancion): Descarga | undefined {
    return descargas.lista.find(
      (d) => d.spotify?.uri === c.uri && (coleccion?.tipo === "track" ? !d.grupo : d.grupo === grupo),
    );
  }

  const paraDescargar = $derived(
    (coleccion?.canciones ?? []).filter((c) => {
      const co = coincidencias[c.posicion];
      const d = descargaDe(c);
      return co && co.grado !== "noEncontrada" && co.elegido && (!d || d.estado.tipo === "cancelada");
    }),
  );

  // Una canción de esta lista ya guardada: con ella se abre su carpeta en el Explorador
  const unaGuardada = $derived(
    (coleccion?.canciones ?? []).map(descargaDe).find((d) => d?.estado.tipo === "completada" && d.guardado),
  );

  onMount(() => {
    leer<Calidad>("formatoSpotify", "audio").then((c) => (calidad = c));
    leer("spotifyGuardar", true).then((g) => (guardarAlTerminar = g));
    const escuchas = [
      listen<{ sesion: number; posicion: number; coincidencia: Coincidencia }>("spotify-resultado", (e) => {
        if (e.payload.sesion === sesion) coincidencias[e.payload.posicion] = e.payload.coincidencia;
      }),
      listen<{ sesion: number }>("spotify-fin-busqueda", (e) => {
        if (e.payload.sesion === sesion) buscando = false;
      }),
    ];
    (async () => {
      try {
        coleccion = await invoke<Coleccion>("leer_spotify", { enlace });
        grupo = gruposSpotify.get(`${coleccion.tipo}-${coleccion.id}`) ?? null;
        await Promise.all(escuchas);
        await invoke("buscar_spotify", { sesion, canciones: coleccion.canciones });
      } catch (e) {
        error = String(e);
        buscando = false;
      }
    })();
    // Al cerrar la pantalla se paran los yt-dlp de la búsqueda
    return () => {
      invoke("cancelar_busqueda_spotify", { sesion });
      escuchas.forEach((p) => p.then((f) => f()));
    };
  });

  async function descargar(canciones: Cancion[]) {
    if (!coleccion || canciones.length === 0) return;
    const peticiones = canciones.map((c) => {
      const co = coincidencias[c.posicion];
      const videos = [co.elegido!, ...co.alternativas].map((r) => `https://www.youtube.com/watch?v=${r.id}`);
      return { cancion: c, videos };
    });
    try {
      const nuevo = await invoke<string | null>("descargar_spotify", {
        coleccion: { tipo: coleccion.tipo, id: coleccion.id, nombre: coleccion.nombre, portada: coleccion.portada },
        canciones: peticiones,
        calidad,
        guardar: guardarAlTerminar,
        grupo,
      });
      if (nuevo) {
        grupo = nuevo;
        gruposSpotify.set(`${coleccion.tipo}-${coleccion.id}`, nuevo);
      }
      mostrarMensaje({ texto: guardarAlTerminar ? t.spotifyAvisoGuardar : t.spotifyAvisoInicio });
    } catch (e) {
      mostrarMensaje({ texto: formato(t.noSePudo, String(e)) });
    }
  }

  function cambiarGuardar() {
    guardarAlTerminar = !guardarAlTerminar;
    guardar("spotifyGuardar", guardarAlTerminar);
  }

  const duracion = (ms: number) => formatearDuracion(ms / 1000);
  const etiquetaGrado = { identica: t.identica, parecida: t.parecida, noEncontrada: t.noEncontrada };
  const resumirError = (m: string) => m.replace(/^\[[^\]]+]\s*[^:\s]+:\s*/, "").split(". ")[0];
</script>

<div class="cabecera">
  <a class="boton-icono" href="/" title={t.atras} aria-label={t.atras}><Icono nombre="atras" /></a>
  <h1>Spotify</h1>
</div>

{#if error}
  <p class="error-grande">{error}</p>
{:else if !coleccion}
  <p class="cargando">{t.spotifyLeyendo}</p>
{:else}
  <section class="coleccion">
    {#if coleccion.portada}
      <img class="portada" src={convertFileSrc(coleccion.portada)} alt="" />
    {/if}
    <div class="datos">
      <h2>{coleccion.nombre}</h2>
      {#if coleccion.autor}<p class="autor">{coleccion.autor}</p>{/if}
      <p class="detalle">
        {total === 1
          ? formato(t.spotifyUnaCancion, calidad === "mp3" ? "MP3" : "M4A")
          : formato(t.spotifyCanciones, total, calidad === "mp3" ? "MP3" : "M4A")}
      </p>
      {#if coleccion.totalReal}
        <p class="aviso">{formato(t.spotifyLimite, coleccion.totalReal)}</p>
      {/if}
    </div>
  </section>

  <section class="estado-busqueda">
    {#if buscando}
      <p>{formato(t.spotifyBuscando, recibidas, total)}</p>
      <div class="barra"><span style:width="{total ? (recibidas / total) * 100 : 0}%"></span></div>
    {:else}
      <p>{formato(t.spotifyResumen, cuenta("identica"), cuenta("parecida"), cuenta("noEncontrada"))}</p>
    {/if}
    <label class="casilla">
      <input type="checkbox" checked={guardarAlTerminar} onchange={cambiarGuardar} />
      {formato(t.spotifyGuardar, coleccion.tipo === "track" ? "" : coleccion.nombre)}
    </label>
    <button class="boton" disabled={buscando || paraDescargar.length === 0} onclick={() => descargar(paraDescargar)}>
      {formato(t.spotifyDescargarTodas, paraDescargar.length)}
    </button>
    {#if unaGuardada}
      <button class="boton tonal" onclick={() => mostrarGuardado(unaGuardada.id)}>
        <Icono nombre="carpeta" tamano={18} />{t.verEnExplorador}
      </button>
    {/if}
  </section>

  <ol class="canciones">
    {#each coleccion.canciones as cancion (cancion.posicion)}
      {@const co = coincidencias[cancion.posicion]}
      {@const d = descargaDe(cancion)}
      <li>
        <button
          class="fila"
          disabled={!co?.elegido}
          title={co?.elegido ? t.spotifyAbrirYoutube : undefined}
          onclick={() => co?.elegido && openUrl(`https://www.youtube.com/watch?v=${co.elegido.id}`)}
        >
          <span class="numero">{cancion.posicion}</span>
          <span class="textos">
            <span class="titulo">{cancion.titulo}</span>
            <span class="detalle">{cancion.artistas.join(", ")} · {duracion(cancion.duracionMs)}</span>
            {#if co?.elegido}
              <span class="detalle">
                ▶ {co.elegido.titulo}{co.elegido.canal ? ` · ${co.elegido.canal}` : ""}{co.elegido.duracion
                  ? ` · ${formatearDuracion(co.elegido.duracion)}`
                  : ""}
              </span>
            {/if}
            {#if d?.estado.tipo === "fallida"}
              <span class="detalle rojo">{resumirError(d.estado.mensaje)}</span>
            {/if}
          </span>
          {#if co}
            <span class="grado {co.grado}">● {etiquetaGrado[co.grado]}</span>
          {:else}
            <span class="grado buscando">…</span>
          {/if}
        </button>
        <span class="accion">
          {#if d && estaActiva(d)}
            {#if d.estado.tipo === "enCola"}
              <span class="detalle" title={t.enCola}>{t.enCola}</span>
            {:else}
              <span class="girando" title={t.descargar}></span>
            {/if}
          {:else if d?.estado.tipo === "completada"}
            <span class="hecho" title={t.completada}><Icono nombre="hecho" tamano={22} /></span>
          {:else if d?.estado.tipo === "fallida"}
            <button class="boton-icono rojo" title={t.reintentar} aria-label={t.reintentar} onclick={() => reintentar(d.id)}>
              <Icono nombre="reintentar" tamano={20} />
            </button>
          {:else if co?.elegido}
            <button
              class="boton-icono"
              title={t.spotifyDescargar}
              aria-label={t.spotifyDescargar}
              onclick={() => descargar([cancion])}
            >
              <Icono nombre="guardar" tamano={20} />
            </button>
          {/if}
        </span>
      </li>
    {/each}
  </ol>
{/if}

<style>
  .cabecera {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0 0 12px -8px;
  }

  h1 {
    margin: 0;
    font-size: 1.4rem;
  }

  .cargando,
  .error-grande {
    margin: 15vh auto;
    text-align: center;
    color: var(--on-surface-variant);
  }

  .error-grande {
    color: var(--error);
  }

  .coleccion {
    display: flex;
    gap: 20px;
    align-items: center;
  }

  .portada {
    width: 140px;
    height: 140px;
    border-radius: 12px;
    object-fit: cover;
    box-shadow: 0 4px 16px rgb(0 0 0 / 0.2);
  }

  .datos h2 {
    margin: 0 0 4px;
    font-size: 1.4rem;
  }

  .autor {
    margin: 0 0 4px;
    font-weight: 500;
  }

  .detalle {
    margin: 0;
    font-size: 0.87rem;
    color: var(--on-surface-variant);
  }

  .aviso {
    margin: 8px 0 0;
    padding: 8px 12px;
    border-radius: 12px;
    background: var(--tertiary-container);
    color: var(--on-tertiary-container);
    font-size: 0.9rem;
  }

  .estado-busqueda {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px 20px;
    margin: 20px 0 12px;
  }

  .estado-busqueda p {
    margin: 0;
    font-weight: 500;
  }

  .estado-busqueda .barra {
    flex-basis: 100%;
    order: 1;
  }

  .barra {
    height: 4px;
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

  .casilla {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 0.9rem;
    cursor: pointer;
  }

  .casilla input {
    width: 18px;
    height: 18px;
    accent-color: var(--primary);
  }

  .estado-busqueda .boton {
    margin-left: auto;
  }

  .canciones {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .canciones li {
    display: flex;
    align-items: center;
    border-bottom: 1px solid var(--outline-variant);
  }

  .fila {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 14px;
    min-width: 0;
    padding: 10px 8px;
    border: none;
    background: transparent;
    text-align: left;
    cursor: pointer;
  }

  .fila:disabled {
    cursor: default;
  }

  .fila:not(:disabled):hover {
    background: color-mix(in srgb, var(--on-surface) 5%, transparent);
  }

  .numero {
    width: 28px;
    text-align: right;
    color: var(--on-surface-variant);
    font-variant-numeric: tabular-nums;
  }

  .textos {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .textos > span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .titulo {
    font-weight: 600;
  }

  .grado {
    flex: none;
    font-size: 0.85rem;
    font-weight: 600;
  }

  .identica {
    color: #2e7d32;
  }

  .parecida {
    color: #b26a00;
  }

  .noEncontrada,
  .rojo {
    color: var(--error);
  }

  :global([data-tema="oscuro"]) .identica {
    color: #81c784;
  }

  :global([data-tema="oscuro"]) .parecida {
    color: #ffb74d;
  }

  .buscando {
    color: var(--on-surface-variant);
  }

  .accion {
    flex: none;
    display: flex;
    justify-content: center;
    width: 64px;
  }

  .hecho {
    color: #2e7d32;
  }

  .girando {
    width: 20px;
    height: 20px;
    border: 3px solid var(--primary-container);
    border-top-color: var(--primary);
    border-radius: 50%;
    animation: girar 0.9s linear infinite;
  }

  @keyframes girar {
    to {
      transform: rotate(360deg);
    }
  }
</style>
