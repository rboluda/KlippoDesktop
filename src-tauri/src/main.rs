// Sin ventana de consola en release (no quitar)
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    klippo_lib::run()
}
