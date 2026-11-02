<!-- Inicio: campo del enlace y, debajo, los vídeos descargados -->
<script lang="ts">
  import { onMount } from "svelte";
  import { guardar, leer } from "$lib/ajustes";
  import DialogoCalidad from "$lib/DialogoCalidad.svelte";
  import { descargar, descargas, type Calidad } from "$lib/descargas.svelte";
  import { extraerUrl } from "$lib/enlaces";
  import Icono from "$lib/Icono.svelte";
  import { t } from "$lib/i18n";
  import TarjetaDescarga from "$lib/TarjetaDescarga.svelte";

  let enlace = $state("");
  let error = $state("");
  let vista = $state<"lista" | "mosaico">("lista");
  /** Enlace esperando a que se elija la calidad */
  let pendiente = $state<string | null>(null);
  let calidadMarcada = $state<Calidad>("mejor");

  onMount(async () => {
    vista = await leer("vista", "lista");
  });

  async function pedirDescarga(evento: SubmitEvent) {
    evento.preventDefault();
    const url = extraerUrl(enlace);
    if (!url) {
      error = t.noEsEnlace;
      return;
    }
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
    {#each descargas.lista as descarga (descarga.id)}
      <TarjetaDescarga {descarga} mosaico={vista === "mosaico"} />
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
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: 12px;
  }
</style>
