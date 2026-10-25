mod backtest;
mod benchmark;
mod market_data;
mod market_types;
mod performance;
mod real_backtest;
mod risk;
// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            real_backtest::run_real_backtest,
            backtest::run_demo_backtest,
            greet,
            market_data::fetch_history
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
