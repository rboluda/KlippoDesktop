<script lang="ts">
  import "../app.css";
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import { comprobarAlAbrir } from "$lib/actualizacion.svelte";
  import AvisoActualizacion from "$lib/AvisoActualizacion.svelte";
  import { empezarAEscuchar } from "$lib/descargas.svelte";
  import Icono from "$lib/Icono.svelte";
  import Mensaje from "$lib/Mensaje.svelte";
  import { idioma, t } from "$lib/i18n";

  let { children } = $props();

  onMount(() => {
    document.documentElement.lang = idioma;
    // La ventana nace oculta para no destellar en blanco: se enseña ya con el tema puesto.
    // Si Klippo arrancó con Windows, se queda en la bandeja.
    invoke<boolean>("arranque_oculto").then((oculto) => {
      if (oculto) return;
      const ventana = getCurrentWindow();
      ventana.show().then(() => ventana.setFocus());
    });
    empezarAEscuchar();
    comprobarAlAbrir();
    // Un enlace del marcador o de un aviso se atiende en Inicio, aunque se esté en Ajustes
    const quitar = [
      listen("enlace-pendiente", () => {
        if (page.url.pathname !== "/") goto("/");
      }),
      // Al pulsar el aviso de "Vídeo descargado"
      listen<string>("abrir-editor", (e) => goto(`/editor/${e.payload}`)),
    ];
    return () => quitar.forEach((q) => q.then((f) => f()));
  });

  // Soltar un archivo en la ventana haría que la página lo abriera en lugar de Klippo
  function noAbrirArchivos(evento: DragEvent) {
    if (evento.dataTransfer?.types.includes("Files")) evento.preventDefault();
  }
</script>

<svelte:window ondragover={noAbrirArchivos} ondrop={noAbrirArchivos} />

<div class="app">
  <header>
    <a class="marca" href="/">
      <img src="/klippo.png" alt="" width="32" height="32" />
      <span>Klippo</span>
    </a>
    {#if page.url.pathname !== "/ajustes"}
      <a class="boton-icono" href="/ajustes" title={t.ajustes} aria-label={t.ajustes}>
        <Icono nombre="ajustes" />
      </a>
    {/if}
  </header>

  <AvisoActualizacion />

  <main>
    <div class="contenido">
      {@render children()}
    </div>
  </main>

  <Mensaje />
</div>

<style>
  .app {
    display: flex;
    flex-direction: column;
    height: 100vh;
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 16px 8px 20px;
  }

  .marca {
    display: flex;
    align-items: center;
    gap: 10px;
    color: var(--on-surface);
    text-decoration: none;
    font-size: 1.35rem;
    font-weight: 700;
    letter-spacing: -0.01em;
  }

  .marca img {
    border-radius: 7px;
  }

  main {
    flex: 1;
    overflow-y: auto;
    padding: 8px 20px 24px;
  }

  /* En pantallas anchas el contenido no se estira de lado a lado */
  header,
  .contenido {
    width: 100%;
    max-width: 1200px;
    margin: 0 auto;
  }
</style>
