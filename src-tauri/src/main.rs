// Prevents additional console window on Windows in release mode
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tracing_subscriber::{fmt, prelude::*, EnvFilter};

mod commands;
mod core;
mod db;
mod models;
mod providers;
mod utils;

fn main() {
    // Initialize logging
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .init();

    tracing::info!("Starting UVCAD application...");

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::sync::start_sync,
            commands::sync::get_sync_status,
            commands::sync::get_file_list,
            commands::sync::resolve_conflict,
            commands::auth::google_auth,
            commands::auth::get_auth_status,
            commands::auth::logout,
            commands::config::get_config,
            commands::config::update_config,
            commands::config::test_smb_connection,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
