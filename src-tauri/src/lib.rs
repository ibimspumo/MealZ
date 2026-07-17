mod app;
pub mod codex;
pub mod domain;
mod ui;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("mealz=info,warn")),
        )
        .try_init()
        .ok();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .setup(|tauri_app| {
            #[cfg(desktop)]
            tauri_app
                .handle()
                .plugin(tauri_plugin_updater::Builder::new().build())?;
            let data_dir = match std::env::var_os("MEALZ_DATA_DIR") {
                Some(override_dir) => {
                    let path = std::path::PathBuf::from(override_dir);
                    if !path.is_absolute() {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidInput,
                            "MEALZ_DATA_DIR muss ein absoluter Pfad sein",
                        )
                        .into());
                    }
                    path
                }
                None => tauri_app.path().app_data_dir()?,
            };
            std::fs::create_dir_all(&data_dir)?;
            let recipe_media_dir = data_dir.join("recipe-media");
            std::fs::create_dir_all(&recipe_media_dir)?;
            tauri_app
                .asset_protocol_scope()
                .allow_directory(&recipe_media_dir, true)?;
            let store = domain::MealzStore::open(data_dir.join("mealz.sqlite3"))?;
            let runtime = app::AppRuntime::new(store, data_dir);
            runtime.start_event_forwarder(tauri_app.handle().clone());
            tauri_app.manage(runtime);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app::get_bootstrap,
            app::list_recipes,
            app::save_recipe,
            app::delete_recipe,
            app::rate_recipe,
            app::get_week_plan,
            app::save_plan_item,
            app::remove_plan_item,
            app::rebuild_shopping_list,
            app::get_shopping_list,
            app::toggle_shopping_item,
            app::add_shopping_item,
            app::delete_shopping_item,
            app::get_profile,
            app::save_profile,
            app::list_memories,
            app::save_memory,
            app::delete_memory,
            app::list_agent_messages,
            app::agent_send,
            app::agent_new_thread,
            app::agent_stop,
            app::agent_capabilities,
            app::complete_onboarding,
            app::restart_onboarding,
            app::get_agent_files,
            app::save_agent_files,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
