<!-- Una lista de YouTube bajada como canciones (al elegir M4A o MP3): las etiquetas que Klippo
     deduce de cada vídeo, en un formulario para corregirlas antes de descargar, con ayuda de
     MusicBrainz para el álbum y para cada canción. Después, igual que con Spotify. -->
<script lang="ts">
  import { onMount } from "svelte";
  import { convertFileSrc, invoke } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-dialog";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { page } from "$app/state";
  import { guardar, leer } from "$lib/ajustes";
  import { descargas, estaActiva, formatearDuracion, mostrarGuardado, reintentar, type Calidad, type Descarga } from "$lib/descargas.svelte";
  import Icono from "$lib/Icono.svelte";
  import { formato, t } from "$lib/i18n";
  import { mostrarMensaje } from "$lib/mensajes.svelte";
  import { gruposSpotify } from "$lib/spotify";

  interface CancionYoutube {
    posicion: number;
    id: string;
    video: string;
    tituloVideo: string;
    canal: string | null;
    duracion: number | null;
    artista: string;
    titulo: string;
    fiable: boolean;
  }
  interface ListaYoutube {
    id: string;
    nombre: string;
    esAlbum: boolean;
    album: string;
    artistaAlbum: string;
    portada: string | null;
    canciones: CancionYoutube[];
    totalReal: number | null;
  }
  interface Lanzamiento {
    id: string;
    titulo: string;
    artista: string;
    fecha: string | null;
    pais: string | null;
    pistas: number | null;
    formato: string | null;
    tipo: string | null;
    miniatura: string | null;
  }
  interface AlbumMb {
    titulo: string;
    artista: string;
    fecha: string | null;
    pistas: { posicion: number; titulo: string; artista: string; duracionMs: number | null }[];
    portada: string | null;
  }
  interface GrabacionMb {
    titulo: string;
    artista: string;
    album: string | null;
    fecha: string | null;
    duracionMs: number | null;
  }
  /** De dónde salen artista y título de una fila */
  type Origen = "youtubeMusic" | "deducido" | "musicbrainz" | "aMano";
  type Fila = CancionYoutube & { origen: Origen };

  const VARIOS = "Various Artists";
  const enlace = page.url.searchParams.get("enlace") ?? "";
  const calidad: Calidad = page.url.searchParams.get("calidad") === "mp3" ? "mp3" : "audio";

  let lista = $state<ListaYoutube | null>(null);
  let error = $state("");
  let filas = $state<Fila[]>([]);
  let album = $state("");
  let artistaAlbum = $state("");
  let fecha = $state("");
  let portada = $state<string | null>(null);
  let guardarAlTerminar = $state(true);
  let paraPlex = $state(false);
  let grupo = $state<string | null>(null);

  // MusicBrainz: álbumes candidatos y la búsqueda de una canción (una fila cada vez)
  let lanzamientos = $state<Lanzamiento[] | null>(null);
  let buscandoAlbum = $state(false);
  let errorMb = $state("");
  let filaAbierta = $state<number | null>(null);
  let grabaciones = $state<GrabacionMb[] | null>(null);
  let buscandoCancion = $state(false);

  onMount(() => {
    leer("spotifyGuardar", true).then((g) => (guardarAlTerminar = g));
    leer("spotifyPlex", false).then((p) => (paraPlex = p));
    (async () => {
      try {
        const leida = await invoke<ListaYoutube>("leer_lista_musica", { enlace });
        lista = leida;
        filas = leida.canciones.map((c) => ({ ...c, origen: c.fiable ? "youtubeMusic" : "deducido" }));
        album = leida.album;
        artistaAlbum = leida.artistaAlbum;
        portada = leida.portada;
        grupo = gruposSpotify.get(`youtube-${leida.id}`) ?? null;
      } catch (e) {
        error = String(e);
      }
    })();
  });

  /** La descarga de una fila (la de esta lista) */
  function descargaDe(f: Fila): Descarga | undefined {
    return descargas.lista.find((d) => d.spotify?.uri === `youtube:${f.id}` && d.grupo === grupo);
  }
  const libre = (f: Fila) => {
    const d = descargaDe(f);
    return !d || d.estado.tipo === "cancelada";
  };
  const paraDescargar = $derived(filas.filter((f) => libre(f) && f.titulo.trim()));
  const unaGuardada = $derived(filas.map(descargaDe).find((d) => d?.estado.tipo === "completada" && d.guardado));
  const deducidas = $derived(filas.filter((f) => f.origen === "deducido").length);

  async function descargar(elegidas: Fila[]) {
    if (!lista || elegidas.length === 0) return;
    // El grupo se decide aquí, antes de pedir nada: dos clics seguidos van a la misma lista
    grupo ??= `musica-${lista.id}-${Date.now()}`;
    gruposSpotify.set(`youtube-${lista.id}`, grupo);
    try {
      grupo = await invoke<string>("descargar_musica_youtube", {
        album: {
          id: lista.id,
          album,
          artistaAlbum,
          fecha: fecha.trim() || null,
          portada,
          total: filas.length,
        },
        canciones: elegidas.map((f) => ({
          posicion: f.posicion,
          id: f.id,
          video: f.video,
          artista: f.artista,
          titulo: f.titulo,
        })),
        calidad,
        guardar: guardarAlTerminar,
        plex: paraPlex,
        grupo,
      });
      gruposSpotify.set(`youtube-${lista.id}`, grupo);
      mostrarMensaje({ texto: guardarAlTerminar ? t.spotifyAvisoGuardar : t.spotifyAvisoInicio });
    } catch (e) {
      mostrarMensaje({ texto: formato(t.noSePudo, String(e)) });
    }
  }

  function cambiarGuardar() {
    guardarAlTerminar = !guardarAlTerminar;
    guardar("spotifyGuardar", guardarAlTerminar);
  }

  function cambiarPlex() {
    paraPlex = !paraPlex;
    guardar("spotifyPlex", paraPlex);
  }

  // ---- Arreglos rápidos ----

  function intercambiar(f: Fila) {
    [f.artista, f.titulo] = [f.titulo, f.artista];
    f.origen = "aMano";
  }

  function mismoArtista() {
    const a = artistaAlbum.trim();
    if (!a || a === VARIOS) return;
    for (const f of filas) if (libre(f) && f.artista !== a) {
      f.artista = a;
      f.origen = "aMano";
    }
  }

  async function cambiarPortada() {
    const ruta = await open({
      multiple: false,
      directory: false,
      filters: [{ name: t.musicaImagenes, extensions: ["jpg", "jpeg", "png", "webp"] }],
    });
    if (typeof ruta !== "string") return;
    try {
      portada = await invoke<string>("portada_propia", { ruta });
    } catch (e) {
      mostrarMensaje({ texto: formato(t.noSePudo, String(e)) });
    }
  }

  // ---- MusicBrainz ----

  async function buscarAlbum() {
    buscandoAlbum = true;
    errorMb = "";
    lanzamientos = null;
    try {
      lanzamientos = await invoke<Lanzamiento[]>("buscar_album_mb", { artista: artistaAlbum, album });
    } catch (e) {
      errorMb = String(e);
    } finally {
      buscandoAlbum = false;
    }
  }

  const normalizar = (s: string) =>
    s
      .normalize("NFD")
      .replace(/\p{M}/gu, "")
      .toLowerCase()
      .replace(/[^\p{L}\p{N}]+/gu, " ")
      .trim();

  /** Parecido de dos títulos de 0 a 1 (uno dentro del otro, o palabras en común) */
  function parecido(a: string, b: string) {
    const [x, y] = [normalizar(a), normalizar(b)];
    if (!x || !y) return 0;
    if (x === y || x.includes(y) || y.includes(x)) return 1;
    const px = new Set(x.split(" "));
    const py = new Set(y.split(" "));
    const comunes = [...px].filter((p) => py.has(p)).length;
    return comunes / Math.max(px.size, py.size);
  }

  async function usarAlbum(l: Lanzamiento) {
    buscandoAlbum = true;
    errorMb = "";
    try {
      const a = await invoke<AlbumMb>("leer_album_mb", { id: l.id });
      album = a.titulo;
      artistaAlbum = a.artista;
      fecha = a.fecha ?? "";
      if (a.portada) portada = a.portada;
      // Cada fila con su pista: en orden si coinciden en número; si no, por título y duración
      const libres = filas.filter(libre);
      const usadas = new Set<number>();
      let completadas = 0;
      for (const f of libres) {
        let pista = a.pistas.length === filas.length ? a.pistas[filas.indexOf(f)] : undefined;
        if (!pista) {
          let mejor = 0;
          for (const p of a.pistas) {
            if (usadas.has(p.posicion)) continue;
            let puntos = Math.max(parecido(f.titulo, p.titulo), parecido(f.tituloVideo, p.titulo) * 0.9);
            if (f.duracion && p.duracionMs && Math.abs(f.duracion - p.duracionMs / 1000) <= 10) puntos += 0.2;
            if (puntos > mejor && puntos >= 0.6) {
              mejor = puntos;
              pista = p;
            }
          }
        }
        if (!pista) continue;
        usadas.add(pista.posicion);
        f.titulo = pista.titulo;
        f.artista = pista.artista || a.artista;
        f.origen = "musicbrainz";
        completadas++;
      }
      lanzamientos = null;
      mostrarMensaje({ texto: formato(t.musicaCompletadas, completadas, libres.length) });
    } catch (e) {
      errorMb = String(e);
    } finally {
      buscandoAlbum = false;
    }
  }

  async function buscarCancion(f: Fila) {
    if (filaAbierta === f.posicion) {
      filaAbierta = null;
      return;
    }
    filaAbierta = f.posicion;
    grabaciones = null;
    buscandoCancion = true;
    try {
      const encontradas = await invoke<GrabacionMb[]>("buscar_cancion_mb", {
        artista: f.artista,
        titulo: f.titulo,
        duracion: f.duracion,
      });
      if (filaAbierta === f.posicion) grabaciones = encontradas;
    } catch (e) {
      if (filaAbierta === f.posicion) {
        grabaciones = [];
        mostrarMensaje({ texto: String(e) });
      }
    } finally {
      buscandoCancion = false;
    }
  }

  function usarGrabacion(f: Fila, g: GrabacionMb) {
    f.titulo = g.titulo;
    f.artista = g.artista;
    f.origen = "musicbrainz";
    filaAbierta = null;
  }

  const anio = (f: string | null) => f?.slice(0, 4) ?? "";
  const etiquetaOrigen: Record<Origen, string> = {
    youtubeMusic: "YouTube Music",
    deducido: t.musicaDeducido,
    musicbrainz: "MusicBrainz",
    aMano: t.musicaAMano,
  };
</script>

<div class="cabecera">
  <a class="boton-icono" href="/" title={t.atras} aria-label={t.atras}><Icono nombre="atras" /></a>
  <h1>{t.musicaTitulo}</h1>
</div>

{#if error}
  <p class="error-grande">{error}</p>
{:else if !lista}
  <p class="cargando">{t.musicaLeyendo}</p>
{:else}
  <section class="coleccion">
    <button class="portada" onclick={cambiarPortada} title={t.musicaPortada} aria-label={t.musicaPortada}>
      {#if portada}
        <img src={convertFileSrc(portada)} alt="" />
      {:else}
        <Icono nombre="audio" tamano={48} />
      {/if}
      <span class="cambiar">{t.musicaPortada}</span>
    </button>
    <div class="datos">
      <p class="detalle">{lista.nombre} · {formato(t.spotifyCanciones, filas.length, calidad === "mp3" ? "MP3" : "M4A")}</p>
      <div class="campos">
        <label class="campo ancho">
          <span>{t.musicaAlbum}</span>
          <input type="text" bind:value={album} />
        </label>
        <label class="campo">
          <span>{t.musicaArtistaAlbum}</span>
          <input type="text" bind:value={artistaAlbum} list="varios" />
          <datalist id="varios"><option value={VARIOS}></option></datalist>
        </label>
        <label class="campo corto">
          <span>{t.musicaAnio}</span>
          <input type="text" bind:value={fecha} inputmode="numeric" />
        </label>
      </div>
      <div class="botones">
        <button class="boton tonal" onclick={buscarAlbum} disabled={buscandoAlbum || !album.trim()} title={t.musicaMbPrivacidad}>
          <Icono nombre="buscar" tamano={18} />{buscandoAlbum ? t.musicaBuscandoMb : t.musicaBuscarMb}
        </button>
        <button class="boton texto" onclick={mismoArtista} disabled={!artistaAlbum.trim() || artistaAlbum.trim() === VARIOS}>
          {t.musicaMismoArtista}
        </button>
      </div>
      {#if lista.totalReal}
        <p class="aviso">{formato(t.musicaLimite, lista.totalReal)}</p>
      {/if}
    </div>
  </section>

  {#if errorMb}
    <p class="detalle rojo">{errorMb}</p>
  {:else if lanzamientos?.length === 0}
    <p class="detalle">{t.musicaSinMb}</p>
  {:else if lanzamientos}
    <ul class="lanzamientos">
      {#each lanzamientos as l (l.id)}
        <li>
          {#if l.miniatura}
            <img src={convertFileSrc(l.miniatura)} alt="" />
          {:else}
            <span class="sin-imagen"><Icono nombre="audio" tamano={24} /></span>
          {/if}
          <span class="textos">
            <span class="titulo">{l.titulo}</span>
            <span class="detalle">
              {[l.artista, anio(l.fecha), l.pais, l.formato, l.tipo, l.pistas ? formato(t.musicaPistas, l.pistas) : null]
                .filter(Boolean)
                .join(" · ")}
            </span>
          </span>
          <button class="boton tonal" disabled={buscandoAlbum} onclick={() => usarAlbum(l)}>{t.usarEste}</button>
        </li>
      {/each}
      <li class="cerrar"><button class="boton texto" onclick={() => (lanzamientos = null)}>{t.cerrar}</button></li>
    </ul>
  {/if}

  <section class="opciones">
    {#if deducidas > 0}
      <p class="detalle">{formato(t.musicaRevisa, deducidas)}</p>
    {/if}
    <label class="casilla">
      <input type="checkbox" checked={guardarAlTerminar} onchange={cambiarGuardar} />
      {formato(t.spotifyGuardar, album.trim() || "YouTube")}
    </label>
    <label class="casilla" title={t.spotifyPlexDetalle}>
      <input type="checkbox" checked={paraPlex} onchange={cambiarPlex} />
      {t.spotifyPlex}
    </label>
    <button class="boton" disabled={paraDescargar.length === 0} onclick={() => descargar(paraDescargar)}>
      {formato(t.spotifyDescargarTodas, paraDescargar.length)}
    </button>
    {#if unaGuardada}
      <button class="boton tonal" onclick={() => mostrarGuardado(unaGuardada.id)}>
        <Icono nombre="carpeta" tamano={18} />{t.verEnExplorador}
      </button>
    {/if}
  </section>

  <ol class="canciones">
    {#each filas as fila (fila.id + fila.posicion)}
      {@const d = descargaDe(fila)}
      {@const editable = libre(fila)}
      <li>
        <span class="numero">{fila.posicion}</span>
        <div class="etiquetas">
          <div class="dos">
            <input
              type="text"
              class="artista"
              bind:value={fila.artista}
              oninput={() => (fila.origen = "aMano")}
              placeholder={t.musicaArtista}
              aria-label={t.musicaArtista}
              disabled={!editable}
            />
            <button class="boton-icono pequeno" title={t.musicaIntercambiar} aria-label={t.musicaIntercambiar} disabled={!editable} onclick={() => intercambiar(fila)}>
              ⇄
            </button>
            <input
              type="text"
              class="titulo-cancion"
              bind:value={fila.titulo}
              oninput={() => (fila.origen = "aMano")}
              placeholder={t.musicaTituloCancion}
              aria-label={t.musicaTituloCancion}
              disabled={!editable}
            />
          </div>
          <button class="video" title={t.spotifyAbrirYoutube} onclick={() => openUrl(fila.video)}>
            ▶ {fila.tituloVideo}{fila.canal ? ` · ${fila.canal}` : ""}{fila.duracion ? ` · ${formatearDuracion(fila.duracion)}` : ""}
          </button>
          {#if d?.estado.tipo === "fallida"}
            <span class="detalle rojo">{d.estado.mensaje.split(". ")[0]}</span>
          {/if}
        </div>
        <span class="origen {fila.origen}">● {etiquetaOrigen[fila.origen]}</span>
        <span class="accion">
          {#if d && estaActiva(d)}
            {#if d.estado.tipo === "enCola"}
              <span class="detalle">{t.enCola}</span>
            {:else}
              <span class="girando"></span>
            {/if}
          {:else if d?.estado.tipo === "completada"}
            <span class="hecho" title={t.completada}><Icono nombre="hecho" tamano={22} /></span>
          {:else if d?.estado.tipo === "fallida"}
            <button class="boton-icono rojo" title={t.reintentar} aria-label={t.reintentar} onclick={() => reintentar(d.id)}>
              <Icono nombre="reintentar" tamano={20} />
            </button>
          {:else}
            <button
              class="boton-icono"
              class:activo={filaAbierta === fila.posicion}
              title={t.musicaBuscarCancionMb}
              aria-label={t.musicaBuscarCancionMb}
              onclick={() => buscarCancion(fila)}
            >
              <Icono nombre="buscar" tamano={20} />
            </button>
            <button
              class="boton-icono"
              title={t.spotifyDescargar}
              aria-label={t.spotifyDescargar}
              disabled={!fila.titulo.trim()}
              onclick={() => descargar([fila])}
            >
              <Icono nombre="guardar" tamano={20} />
            </button>
          {/if}
        </span>
        {#if filaAbierta === fila.posicion}
          <div class="mb-cancion">
            {#if buscandoCancion}
              <p class="detalle">{t.musicaBuscandoMb}</p>
            {:else if grabaciones?.length === 0}
              <p class="detalle">{t.sinResultados}</p>
            {:else if grabaciones}
              <ul>
                {#each grabaciones as g, i (i)}
                  <li>
                    <span class="textos">
                      <span class="titulo">{g.artista} - {g.titulo}</span>
                      <span class="detalle">
                        {[g.album, anio(g.fecha), g.duracionMs ? formatearDuracion(g.duracionMs / 1000) : null]
                          .filter(Boolean)
                          .join(" · ")}
                      </span>
                    </span>
                    <button class="boton tonal" onclick={() => usarGrabacion(fila, g)}>{t.usarEste}</button>
                  </li>
                {/each}
              </ul>
            {/if}
          </div>
        {/if}
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
    align-items: flex-start;
  }

  .portada {
    position: relative;
    flex: none;
    display: grid;
    place-items: center;
    width: 160px;
    height: 160px;
    padding: 0;
    border: none;
    border-radius: 12px;
    overflow: hidden;
    background: var(--surface-container-high, var(--secondary-container));
    color: var(--on-surface-variant);
    box-shadow: 0 4px 16px rgb(0 0 0 / 0.2);
    cursor: pointer;
  }

  .portada img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .portada .cambiar {
    position: absolute;
    inset: auto 0 0;
    padding: 6px;
    background: rgb(0 0 0 / 0.55);
    color: #fff;
    font-size: 0.8rem;
    opacity: 0;
    transition: opacity 0.15s;
  }

  .portada:hover .cambiar,
  .portada:focus-visible .cambiar {
    opacity: 1;
  }

  .datos {
    flex: 1;
    min-width: 0;
  }

  .detalle {
    margin: 0;
    font-size: 0.87rem;
    color: var(--on-surface-variant);
  }

  .detalle.rojo {
    color: var(--error);
  }

  .campos {
    display: flex;
    flex-wrap: wrap;
    gap: 10px 12px;
    margin: 10px 0;
  }

  .campo {
    display: flex;
    flex-direction: column;
    gap: 2px;
    flex: 1 1 200px;
    font-size: 0.8rem;
    color: var(--on-surface-variant);
  }

  .campo.ancho {
    flex-basis: 100%;
  }

  .campo.corto {
    flex: 0 0 110px;
    max-width: 110px;
  }

  .campo input {
    width: 100%;
    box-sizing: border-box;
  }

  input[type="text"] {
    min-width: 0;
    height: 36px;
    padding: 0 12px;
    border: 1px solid var(--outline-variant);
    border-radius: 8px;
    background: var(--surface);
    color: var(--on-surface);
    font: inherit;
    font-size: 0.95rem;
  }

  input[type="text"]:focus {
    outline: 2px solid var(--primary);
    outline-offset: -1px;
  }

  input[type="text"]:disabled {
    border-color: transparent;
    background: transparent;
  }

  .campo.ancho input {
    font-size: 1.15rem;
    font-weight: 600;
  }

  .botones {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .aviso {
    margin: 8px 0 0;
    padding: 8px 12px;
    border-radius: 12px;
    background: var(--tertiary-container);
    color: var(--on-tertiary-container);
    font-size: 0.9rem;
  }

  .lanzamientos {
    margin: 12px 0 0;
    padding: 8px;
    list-style: none;
    border-radius: 12px;
    background: color-mix(in srgb, var(--on-surface) 4%, transparent);
  }

  .lanzamientos li {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 4px;
  }

  .lanzamientos img,
  .sin-imagen {
    flex: none;
    width: 48px;
    height: 48px;
    border-radius: 6px;
    object-fit: cover;
  }

  .sin-imagen {
    display: grid;
    place-items: center;
    background: var(--secondary-container);
  }

  .lanzamientos .cerrar {
    justify-content: flex-end;
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

  .opciones {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px 20px;
    margin: 20px 0 12px;
  }

  .opciones > .detalle {
    flex-basis: 100%;
  }

  .casilla {
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
  }

  .casilla input {
    width: 18px;
    height: 18px;
    accent-color: var(--primary);
  }

  .opciones > .boton:first-of-type {
    margin-left: auto;
  }

  .canciones {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .canciones > li {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 12px;
    padding: 8px 0;
    border-bottom: 1px solid var(--outline-variant);
  }

  .numero {
    width: 28px;
    text-align: right;
    color: var(--on-surface-variant);
    font-variant-numeric: tabular-nums;
  }

  .etiquetas {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .dos {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .dos .artista {
    flex: 2;
  }

  .dos .titulo-cancion {
    flex: 3;
    font-weight: 600;
  }

  .pequeno {
    width: 32px;
    height: 32px;
    font-size: 1rem;
  }

  .video {
    align-self: flex-start;
    max-width: 100%;
    padding: 0 12px;
    border: none;
    background: transparent;
    color: var(--on-surface-variant);
    font: inherit;
    font-size: 0.82rem;
    text-align: left;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    cursor: pointer;
  }

  .video:hover {
    color: var(--primary);
  }

  .origen {
    flex: none;
    width: 120px;
    font-size: 0.8rem;
    font-weight: 600;
    text-align: right;
  }

  .youtubeMusic,
  .musicbrainz {
    color: #2e7d32;
  }

  .deducido {
    color: #b26a00;
  }

  .aMano {
    color: var(--primary);
  }

  :global([data-tema="oscuro"]) .youtubeMusic,
  :global([data-tema="oscuro"]) .musicbrainz {
    color: #81c784;
  }

  :global([data-tema="oscuro"]) .deducido {
    color: #ffb74d;
  }

  .accion {
    flex: none;
    display: flex;
    align-items: center;
    justify-content: flex-end;
    width: 96px;
    padding-right: 8px;
  }

  .boton-icono.activo {
    background: var(--secondary-container);
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

  .mb-cancion {
    flex-basis: 100%;
    padding: 0 8px 6px 40px;
  }

  .mb-cancion ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .mb-cancion li {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 4px 0;
  }
</style>
