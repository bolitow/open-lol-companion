// Pas de console en plus de la fenêtre sur Windows en release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    olc_desktop_lib::run()
}
