<!-- Inicio: campo del enlace y, debajo, los vídeos descargados -->
<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { guardar, leer } from "$lib/ajustes";
  import DialogoCalidad from "$lib/DialogoCalidad.svelte";
  import { agrupar, descargar, descargas, type Calidad } from "$lib/descargas.svelte";
  import TarjetaLista from "$lib/TarjetaLista.svelte";
  import { esSoportada, extraerUrl } from "$lib/enlaces";
  import Icono from "$lib/Icono.svelte";
  import { t } from "$lib/i18n";
  import TarjetaDescarga from "$lib/TarjetaDescarga.svelte";
  import { actualizarMotor, cargarVersiones, DIAS_MOTOR_ANTIGUO, diasMotor, motor } from "$lib/motor.svelte";
  import { formato } from "$lib/i18n";

  let enlace = $state("");
  let error = $state("");
  let vista = $state<"lista" | "mosaico">("lista");
  /** Enlace esperando a que se elija la calidad */
  let pendiente = $state<string | null>(null);
  let calidadMarcada = $state<Calidad>("mejor");
  /** Arrastrando un enlace sobre la ventana */
  let arrastrando = $state(false);
  /** El último enlace puesto solo en el campo: no se vuelve a poner si el usuario lo borra */
  let ultimoAutorrelleno = "";

  onMount(() => {
    leer<"lista" | "mosaico">("vista", "lista").then((v) => (vista = v));
    cargarVersiones();
    tomarPendiente();
    autorrellenar();
    const quitar = [
      // Copiado con Klippo delante: al campo
      listen<string>("enlace-copiado", (e) => ponerEnlace(e.payload)),
      // Llegado por el marcador o por el botón Descargar de un aviso
      listen("enlace-pendiente", tomarPendiente),
      // Al volver a Klippo, lo que haya en el portapapeles (como en Android)
      getCurrentWindow().onFocusChanged(({ payload: enfocada }) => enfocada && autorrellenar()),
    ];
    return () => quitar.forEach((p) => p.then((f) => f()));
  });

  function ponerEnlace(url: string) {
    // No se pisa lo que el usuario esté escribiendo
    if (enlace.trim() && !extraerUrl(enlace)) return;
    enlace = url;
    error = "";
  }

  async function autorrellenar() {
    if (enlace.trim() || pendiente) return;
    const url = await invoke<string | null>("enlace_del_portapapeles");
    if (!url || !esSoportada(url) || url === ultimoAutorrelleno) return;
    if (descargas.lista.some((d) => d.url === url)) return;
    ultimoAutorrelleno = url;
    ponerEnlace(url);
  }

  async function tomarPendiente() {
    const url = await invoke<string | null>("tomar_enlace_pendiente");
    if (!url) return;
    enlace = url;
    // Los enlaces de otras webs se quedan en el campo para que el usuario decida
    if (esSoportada(url)) pedir(url);
  }

  async function pegar() {
    const url = await invoke<string | null>("enlace_del_portapapeles");
    if (url) ponerEnlace(url);
  }

  // Ctrl+V fuera del campo: pegar y descargar de una vez
  async function alPulsarTecla(evento: KeyboardEvent) {
    const enCampo = evento.target instanceof HTMLInputElement || evento.target instanceof HTMLSelectElement;
    if (!evento.ctrlKey || evento.key.toLowerCase() !== "v" || enCampo || pendiente) return;
    evento.preventDefault();
    const url = await invoke<string | null>("enlace_del_portapapeles");
    if (url) {
      enlace = url;
      pedir(url);
    }
  }

  // Arrastrar la pestaña o un enlace desde el navegador hasta la ventana
  function alArrastrar(evento: DragEvent) {
    const tipos = evento.dataTransfer?.types ?? [];
    if (tipos.includes("text/uri-list") || tipos.includes("text/plain")) {
      evento.preventDefault();
      arrastrando = true;
    }
  }

  function alSoltar(evento: DragEvent) {
    evento.preventDefault();
    arrastrando = false;
    const datos = evento.dataTransfer;
    const url = extraerUrl(datos?.getData("text/uri-list") || datos?.getData("text/plain"));
    if (url) {
      enlace = url;
      pedir(url);
    }
  }

  function pedirDescarga(evento: SubmitEvent) {
    evento.preventDefault();
    const url = extraerUrl(enlace);
    if (!url) {
      error = t.noEsEnlace;
      return;
    }
    pedir(url);
  }

  async function pedir(url: string) {
    error = "";
    calidadMarcada = await leer<Calidad>("calidad", "mejor");
    if (await leer("preguntarCalidad", true)) {
      pendiente = url;
    } else {
      empezar(url, calidadMarcada);
    }
  }

  async function empezar(url: string, calidad: Calidad) {
    // La elegida queda marcada para la próxima
    guardar("calidad", calidad);
    try {
      await descargar(url, calidad);
      enlace = "";
    } catch (e) {
      error = String(e);
    }
  }

  function cambiarVista(nueva: "lista" | "mosaico") {
    vista = nueva;
    guardar("vista", nueva);
  }
</script>

<svelte:window
  onkeydown={alPulsarTecla}
  ondragover={alArrastrar}
  ondragleave={(e) => e.relatedTarget === null && (arrastrando = false)}
  ondrop={alSoltar}
/>

{#if (diasMotor() ?? 0) > DIAS_MOTOR_ANTIGUO}
  <div class="motor-antiguo">
    <span>{formato(t.motorAntiguo, diasMotor() ?? 0)}</span>
    <button class="boton texto" onclick={actualizarMotor} disabled={motor.actualizando}>
      {motor.actualizando ? t.actualizandoMotor : t.actualizar}
    </button>
  </div>
{/if}

{#if arrastrando}
  <div class="soltar" aria-hidden="true">
    <Icono nombre="enlace" tamano={48} />
    <p>{t.sueltaEnlace}</p>
  </div>
{/if}

<form class="entrada" onsubmit={pedirDescarga}>
  <div class="campo" class:con-error={error}>
    <Icono nombre="enlace" />
    <input
      type="text"
      bind:value={enlace}
      oninput={() => (error = "")}
      placeholder={t.campoEnlace}
      title={t.campoAyuda}
      spellcheck="false"
      autocomplete="off"
    />
    {#if enlace}
      <button
        type="button"
        class="boton-icono"
        title={t.borrar}
        aria-label={t.borrar}
        onclick={() => ((enlace = ""), (error = ""))}
      >
        <Icono nombre="cerrar" tamano={20} />
      </button>
    {:else}
      <button type="button" class="boton-icono" title={t.pegar} aria-label={t.pegar} onclick={pegar}>
        <Icono nombre="pegar" tamano={20} />
      </button>
    {/if}
  </div>
  <button type="submit" class="boton" disabled={!enlace.trim()}>{t.descargar}</button>
</form>
{#if error}
  <p class="error">{error}</p>
{/if}

{#if descargas.lista.length === 0}
  <section class="vacia">
    <img src="/klippo.png" alt="" width="96" height="96" />
    <h2>{t.listaVacia}</h2>
    <p>{t.listaVaciaDetalle}</p>
  </section>
{:else}
  <div class="cabecera-lista">
    <h2>{t.tusVideos}</h2>
    <div class="vistas">
      <button
        class="boton-icono"
        class:activa={vista === "lista"}
        title={t.vistaLista}
        aria-label={t.vistaLista}
        onclick={() => cambiarVista("lista")}
      >
        <Icono nombre="lista" tamano={20} />
      </button>
      <button
        class="boton-icono"
        class:activa={vista === "mosaico"}
        title={t.vistaMosaico}
        aria-label={t.vistaMosaico}
        onclick={() => cambiarVista("mosaico")}
      >
        <Icono nombre="mosaico" tamano={20} />
      </button>
    </div>
  </div>
  <section class={vista}>
    {#each agrupar(descargas.lista) as entrada (entrada.tipo === "lista" ? `lista-${entrada.grupo}` : entrada.descarga.id)}
      {#if entrada.tipo === "lista"}
        <TarjetaLista grupo={entrada.grupo} elementos={entrada.elementos} mosaico={vista === "mosaico"} />
      {:else}
        <TarjetaDescarga descarga={entrada.descarga} mosaico={vista === "mosaico"} />
      {/if}
    {/each}
  </section>
{/if}

{#if pendiente}
  <DialogoCalidad
    marcada={calidadMarcada}
    onelegir={(calidad) => empezar(pendiente!, calidad)}
    oncerrar={() => (pendiente = null)}
  />
{/if}

<style>
  .motor-antiguo {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin: 0 4px 8px 16px;
    font-size: 0.87rem;
    color: var(--on-surface-variant);
  }

  .soltar {
    position: fixed;
    inset: 12px;
    z-index: 10;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    border: 3px dashed var(--primary);
    border-radius: 24px;
    background: color-mix(in srgb, var(--surface) 92%, transparent);
    color: var(--primary);
    font-size: 1.2rem;
    font-weight: 600;
    pointer-events: none;
  }

  .entrada {
    display: flex;
    gap: 12px;
    align-items: center;
  }

  .campo {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 10px;
    height: 52px;
    padding: 0 6px 0 16px;
    border-radius: 26px;
    background: var(--surface-container-high);
    color: var(--on-surface-variant);
  }

  .campo:focus-within {
    outline: 2px solid var(--primary);
  }

  .campo.con-error {
    outline: 2px solid var(--error);
  }

  .campo input {
    flex: 1;
    min-width: 0;
    border: none;
    outline: none;
    background: transparent;
    color: var(--on-surface);
    user-select: text;
  }

  .entrada .boton {
    height: 52px;
    border-radius: 26px;
  }

  .error {
    margin: 8px 16px 0;
    font-size: 0.87rem;
    color: var(--error);
  }

  .vacia {
    max-width: 440px;
    margin: 12vh auto 0;
    text-align: center;
    color: var(--on-surface-variant);
  }

  .vacia img {
    border-radius: 22px;
    opacity: 0.9;
  }

  .vacia h2 {
    margin: 20px 0 8px;
    font-size: 1.15rem;
    color: var(--on-surface);
  }

  .vacia p {
    margin: 0;
  }

  .cabecera-lista {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin: 20px 0 8px 4px;
  }

  .cabecera-lista h2 {
    margin: 0;
    font-size: 0.9rem;
    font-weight: 600;
    color: var(--primary);
  }

  .vistas {
    display: flex;
    gap: 2px;
  }

  .vistas .activa {
    background: var(--secondary-container);
    color: var(--on-secondary-container);
  }

  .lista {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .mosaico {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(270px, 1fr));
    gap: 12px;
  }
</style>
