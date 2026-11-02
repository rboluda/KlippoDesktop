<!-- Una descarga en la lista o en el mosaico: miniatura, título, estado y acciones -->
<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import {
    cancelar,
    eliminar,
    estaActiva,
    formatearDuracion,
    mostrarEnCarpeta,
    pideSesion,
    reintentar,
    reproducir,
    resumirError,
    type Descarga,
  } from "./descargas.svelte";
  import { plataforma } from "./enlaces";
  import Icono from "./Icono.svelte";
  import { formato, nombreCalidad, t } from "./i18n";
  import { actualizarMotor, motor } from "./motor.svelte";

  let { descarga, mosaico = false }: { descarga: Descarga; mosaico?: boolean } = $props();

  let errorCompleto = $state(false);

  const activa = $derived(estaActiva(descarga));
  const completada = $derived(descarga.estado.tipo === "completada");
  const porcentaje = $derived(Math.round((descarga.progreso ?? 0) * 100));

  const textoEstado = $derived.by(() => {
    const e = descarga.estado;
    switch (e.tipo) {
      case "analizando":
        return t.analizando;
      case "descargando":
        if (descarga.progreso === null) return t.analizando;
        return descarga.eta
          ? formato(t.progresoEta, porcentaje, formatearDuracion(descarga.eta))
          : `${porcentaje} %`;
      case "procesando":
        return t.procesando;
      case "completada":
        return [plataforma(descarga.url), nombreCalidad(descarga.calidad)].join(" · ");
      case "cancelada":
        return t.cancelada;
      case "fallida":
        return formato(t.fallida, errorCompleto ? e.mensaje : resumirError(e.mensaje));
    }
  });

  async function actualizarYReintentar() {
    if (await actualizarMotor()) reintentar(descarga.id);
  }

  function abrir() {
    if (completada) reproducir(descarga.id);
  }
</script>

<article class="tarjeta" class:mosaico class:fallida={descarga.estado.tipo === "fallida"}>
  <button class="miniatura" onclick={abrir} disabled={!completada} title={completada ? t.reproducir : undefined}>
    {#if descarga.miniatura}
      <img src={convertFileSrc(descarga.miniatura)} alt="" loading="lazy" />
    {:else}
      <span class="sin-imagen">
        <Icono nombre={!completada ? "enlace" : descarga.esAudio ? "audio" : "reproducir"} tamano={32} />
      </span>
    {/if}
    {#if descarga.duracion}
      <span class="duracion">{formatearDuracion(descarga.duracion)}</span>
    {/if}
    {#if descarga.esAudio && completada}
      <span class="etiqueta-audio"><Icono nombre="audio" tamano={14} />{t.audio}</span>
    {/if}
  </button>

  <div class="cuerpo">
    <p class="titulo" title={descarga.titulo ?? descarga.url}>{descarga.titulo ?? descarga.url}</p>
    {#if descarga.estado.tipo === "fallida"}
      <button class="estado error" onclick={() => (errorCompleto = !errorCompleto)}>{textoEstado}</button>
      {#if pideSesion(descarga.estado.mensaje)}
        <p class="estado">{t.pideSesion}</p>
      {/if}
    {:else}
      <p class="estado">{textoEstado}</p>
    {/if}
    {#if activa}
      <div class="barra" class:indeterminada={descarga.progreso === null}>
        <span style:width={descarga.progreso === null ? undefined : `${porcentaje}%`}></span>
      </div>
    {/if}
  </div>

  <div class="acciones">
    {#if activa}
      <button class="boton-icono" title={t.cancelar} aria-label={t.cancelar} onclick={() => cancelar(descarga.id)}>
        <Icono nombre="cerrar" tamano={20} />
      </button>
    {:else}
      {#if completada}
        <button class="boton-icono" title={t.reproducir} aria-label={t.reproducir} onclick={abrir}>
          <Icono nombre="reproducir" tamano={20} />
        </button>
        <button
          class="boton-icono"
          title={t.mostrarEnCarpeta}
          aria-label={t.mostrarEnCarpeta}
          onclick={() => mostrarEnCarpeta(descarga.id)}
        >
          <Icono nombre="carpeta" tamano={20} />
        </button>
      {:else}
        <button class="boton-icono" title={t.reintentar} aria-label={t.reintentar} onclick={() => reintentar(descarga.id)}>
          <Icono nombre="reintentar" tamano={20} />
        </button>
      {/if}
      <button class="boton-icono" title={t.eliminar} aria-label={t.eliminar} onclick={() => eliminar(descarga.id)}>
        <Icono nombre="eliminar" tamano={20} />
      </button>
    {/if}
  </div>

  {#if descarga.estado.tipo === "fallida"}
    <button class="boton texto actualizar" onclick={actualizarYReintentar} disabled={motor.actualizando}>
      {motor.actualizando ? t.actualizandoMotor : t.actualizarYReintentar}
    </button>
  {/if}
</article>

<style>
  .tarjeta {
    display: grid;
    grid-template-columns: 160px 1fr auto;
    grid-template-areas:
      "miniatura cuerpo acciones"
      "miniatura actualizar actualizar";
    grid-template-rows: 1fr auto;
    column-gap: 16px;
    padding: 8px;
    border-radius: 16px;
    background: var(--surface-container-low);
  }

  .miniatura {
    grid-area: miniatura;
    position: relative;
    aspect-ratio: 16 / 9;
    padding: 0;
    border: none;
    border-radius: 10px;
    overflow: hidden;
    background: var(--surface-container-highest);
    color: var(--on-surface-variant);
    cursor: pointer;
    align-self: start;
  }

  .miniatura:disabled {
    cursor: default;
  }

  .miniatura img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }

  .sin-imagen {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
  }

  .duracion,
  .etiqueta-audio {
    position: absolute;
    bottom: 6px;
    padding: 1px 6px;
    border-radius: 6px;
    background: rgb(0 0 0 / 0.7);
    color: #fff;
    font-size: 0.75rem;
    font-weight: 600;
  }

  .duracion {
    right: 6px;
  }

  .etiqueta-audio {
    left: 6px;
    display: flex;
    align-items: center;
    gap: 3px;
  }

  .cuerpo {
    grid-area: cuerpo;
    min-width: 0;
    align-self: center;
  }

  .titulo {
    margin: 0 0 4px;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    word-break: break-word;
  }

  .estado {
    display: block;
    margin: 0;
    padding: 0;
    border: none;
    background: none;
    font-size: 0.87rem;
    text-align: left;
    color: var(--on-surface-variant);
    user-select: text;
  }

  .estado.error {
    color: var(--error);
    cursor: pointer;
  }

  .barra {
    height: 4px;
    margin-top: 10px;
    border-radius: 2px;
    background: var(--primary-container);
    overflow: hidden;
  }

  .barra span {
    display: block;
    height: 100%;
    border-radius: 2px;
    background: var(--primary);
    transition: width 0.2s linear;
  }

  .barra.indeterminada span {
    width: 30%;
    animation: ir-y-volver 1.2s ease-in-out infinite;
  }

  @keyframes ir-y-volver {
    from {
      transform: translateX(-100%);
    }
    to {
      transform: translateX(340%);
    }
  }

  .acciones {
    grid-area: acciones;
    display: flex;
    align-items: center;
  }

  .actualizar {
    grid-area: actualizar;
    justify-self: start;
    height: 32px;
    margin-left: -12px;
  }

  /* Mosaico: miniatura arriba, título y acciones debajo */
  .tarjeta.mosaico {
    grid-template-columns: 1fr;
    grid-template-areas: "miniatura" "cuerpo" "acciones" "actualizar";
    grid-template-rows: auto 1fr auto auto;
    row-gap: 8px;
  }

  .mosaico .miniatura {
    width: 100%;
  }

  .mosaico .cuerpo {
    align-self: start;
    padding: 0 4px;
  }

  .mosaico .acciones {
    justify-content: flex-end;
  }
</style>
