<!-- Elegir un emoji: los más usados de un clic, u otro escrito (Windows + . abre el panel de
     emojis de Windows) -->
<script lang="ts">
  import Dialogo from "../Dialogo.svelte";
  import { t } from "../i18n";
  import { FRECUENTES } from "./modelo";

  let { onelegir, oncerrar }: { onelegir: (emoji: string) => void; oncerrar: () => void } = $props();

  let otro = $state("");

  /** Primer carácter visible del texto (un emoji puede ocupar varios: 👨‍👩‍👧, 👍🏽…) */
  function primerGrafema(texto: string): string {
    const segmentos = new Intl.Segmenter(undefined, { granularity: "grapheme" }).segment(texto.trim());
    return segmentos[Symbol.iterator]().next().value?.segment ?? "";
  }

  let elegido = $state("");
</script>

<Dialogo
  titulo={t.emojiTitulo}
  aceptar={t.anadir}
  puedeAceptar={primerGrafema(otro).length > 0}
  onaceptar={() => onelegir(primerGrafema(otro))}
  {oncerrar}
>
  <div class="rejilla">
    {#each FRECUENTES as emoji (emoji)}
      <button
        type="button"
        class:elegido={elegido === emoji}
        onclick={() => {
          elegido = emoji;
          otro = emoji;
        }}
        ondblclick={() => onelegir(emoji)}>{emoji}</button
      >
    {/each}
  </div>
  <input type="text" bind:value={otro} placeholder={t.emojiOtro} maxlength="16" />
</Dialogo>

<style>
  .rejilla {
    display: grid;
    grid-template-columns: repeat(8, 1fr);
    gap: 4px;
    margin-bottom: 16px;
  }

  .rejilla button {
    aspect-ratio: 1;
    border: none;
    border-radius: 10px;
    background: transparent;
    font-size: 1.6rem;
    font-family: "Segoe UI Emoji", sans-serif;
    cursor: pointer;
  }

  .rejilla button:hover {
    background: color-mix(in srgb, var(--on-surface) 8%, transparent);
  }

  .rejilla button.elegido {
    background: var(--secondary-container);
  }
</style>
