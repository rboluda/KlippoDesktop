<!-- Añadir texto: lo que se escribe y su estilo (contorno o fondo) -->
<script lang="ts">
  import Dialogo from "../Dialogo.svelte";
  import { t } from "../i18n";
  import type { EstiloTexto } from "./modelo";

  let { onaceptar, oncerrar }: { onaceptar: (texto: string, estilo: EstiloTexto) => void; oncerrar: () => void } =
    $props();

  let texto = $state("");
  let estilo = $state<EstiloTexto>("contorno");
</script>

<Dialogo
  titulo={t.textoTitulo}
  aceptar={t.anadir}
  puedeAceptar={texto.trim().length > 0}
  onaceptar={() => onaceptar(texto.trim(), estilo)}
  {oncerrar}
>
  <!-- svelte-ignore a11y_autofocus -->
  <input type="text" bind:value={texto} placeholder={t.textoCampo} maxlength="80" autofocus />
  <div class="estilos">
    <label class:elegido={estilo === "contorno"}>
      <input type="radio" bind:group={estilo} value="contorno" />
      <span class="muestra contorno">Aa</span>{t.textoContorno}
    </label>
    <label class:elegido={estilo === "fondo"}>
      <input type="radio" bind:group={estilo} value="fondo" />
      <span class="muestra fondo">Aa</span>{t.textoFondo}
    </label>
  </div>
</Dialogo>

<style>
  .estilos {
    display: flex;
    gap: 8px;
    margin-top: 16px;
  }

  label {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
    border: 1px solid var(--outline-variant);
    border-radius: 12px;
    cursor: pointer;
  }

  label.elegido {
    border-color: var(--primary);
    background: var(--secondary-container);
  }

  input[type="radio"] {
    display: none;
  }

  .muestra {
    padding: 0 6px;
    border-radius: 4px;
    font-weight: 800;
    color: #fff;
  }

  .muestra.contorno {
    -webkit-text-stroke: 1px #000;
    paint-order: stroke fill;
    background: #8a8a8a;
  }

  .muestra.fondo {
    background: rgb(0 0 0 / 0.6);
  }
</style>
