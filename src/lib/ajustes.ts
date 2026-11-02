// Ajustes de Klippo en %APPDATA%\es.rboluda.klippo.desktop\ajustes.json
import { load, type Store } from "@tauri-apps/plugin-store";

let almacen: Promise<Store> | null = null;

function abrir(): Promise<Store> {
  return (almacen ??= load("ajustes.json", { autoSave: true, defaults: {} }));
}

export async function leer<T>(clave: string, porDefecto: T): Promise<T> {
  const valor = await (await abrir()).get<T>(clave);
  return valor ?? porDefecto;
}

export async function guardar(clave: string, valor: unknown): Promise<void> {
  await (await abrir()).set(clave, valor);
}
