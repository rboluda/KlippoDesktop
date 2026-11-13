<!-- Pegar una lista de canciones en texto (una por línea, "Artista - Título"): se buscan en
     YouTube y se bajan en la misma pantalla que las de Spotify -->
<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import Icono from "$lib/Icono.svelte";
  import { formato, t } from "$lib/i18n";
  import { guardarListaDeTexto, listaDeTexto, PREFIJO_TEXTO, textoPegado } from "$lib/spotify";

  interface Cancion {
    titulo: string;
    artistas: string[];
  }

  // Editando una lista ya buscada: se conserva su id (y con él su tarjeta)
  const id = page.url.searchParams.get("id") ?? undefined;

  let texto = $state(textoPegado.valor);
  let nombre = $state("");
  let canciones = $state<Cancion[]>([]);
  textoPegado.valor = "";

  onMount(async () => {
    if (!id) return;
    const lista = await listaDeTexto(id);
    if (lista) {
      texto = lista.texto;
      nombre = lista.nombre;
    }
  });

  // Lo que Klippo entiende del texto, al momento (sin red)
  $effect(() => {
    const actual = texto;
    invoke<{ canciones: Cancion[] }>("leer_lista_de_texto", { texto: actual, id: "", nombre: "" }).then((c) => {
      if (actual === texto) canciones = c.canciones;
    });
  });

  async function buscar(evento: SubmitEvent) {
    evento.preventDefault();
    if (canciones.length === 0) return;
    const clave = await guardarListaDeTexto(texto, nombre.trim() || t.listaTextoNombrePorDefecto, id);
    goto(`/spotify?enlace=${encodeURIComponent(PREFIJO_TEXTO + clave)}`, { replaceState: !!id });
  }

  const ejemplo = $derived(canciones[0]);
</script>

<div class="cabecera">
  <a class="boton-icono" href="/" title={t.atras} aria-label={t.atras}><Icono nombre="atras" /></a>
  <h1>{t.listaTextoTitulo}</h1>
</div>

<p class="ayuda">{t.listaTextoAyuda}</p>

<form onsubmit={buscar}>
  <label class="campo-nombre">
    <span>{t.listaTextoNombre}</span>
    <input type="text" bind:value={nombre} placeholder={t.listaTextoNombrePorDefecto} spellcheck="false" />
  </label>
  <!-- svelte-ignore a11y_autofocus -->
  <textarea
    bind:value={texto}
    placeholder={t.listaTextoPista}
    aria-label={t.listaTextoTitulo}
    spellcheck="false"
    autofocus
    rows="14"
  ></textarea>
  <div class="pie">
    <p class="detalle">
      {#if canciones.length === 0}
        {t.listaTextoNinguna}
      {:else}
        {canciones.length === 1 ? t.listaTextoUna : formato(t.listaTextoVarias, canciones.length)}
        {#if ejemplo}
          · {ejemplo.artistas.length
            ? formato(t.listaTextoEjemplo, ejemplo.artistas.join(", "), ejemplo.titulo)
            : formato(t.listaTextoEjemploSinArtista, ejemplo.titulo)}
        {/if}
      {/if}
    </p>
    <button class="boton" type="submit" disabled={canciones.length === 0}>{t.listaTextoBuscar}</button>
  </div>
</form>

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

  .ayuda {
    margin: 0 0 16px;
    color: var(--on-surface-variant);
  }

  form {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .campo-nombre {
    display: flex;
    align-items: center;
    gap: 12px;
    font-weight: 500;
  }

  input,
  textarea {
    border: 1px solid var(--outline);
    background: var(--surface);
    color: var(--on-surface);
    font: inherit;
  }

  input {
    flex: 1;
    max-width: 420px;
    height: 40px;
    padding: 0 16px;
    border-radius: 20px;
  }

  textarea {
    min-height: 240px;
    padding: 12px 16px;
    border-radius: 16px;
    resize: vertical;
    line-height: 1.5;
  }

  input:focus,
  textarea:focus {
    outline: 2px solid var(--primary);
    outline-offset: -1px;
  }

  .pie {
    display: flex;
    align-items: center;
    gap: 16px;
  }

  .detalle {
    flex: 1;
    margin: 0;
    font-size: 0.9rem;
    color: var(--on-surface-variant);
  }
</style>
