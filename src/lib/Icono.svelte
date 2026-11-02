<script lang="ts">
  import { iconos } from "./iconos";

  let { nombre, tamano = 24 }: { nombre: keyof typeof iconos; tamano?: number } = $props();
</script>

<svg width={tamano} height={tamano} viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
  <path d={iconos[nombre]} />
</svg>
