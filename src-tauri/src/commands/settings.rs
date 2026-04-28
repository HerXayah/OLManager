use log::info;
use tauri::Manager as TauriManager;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AppSettings {
    pub theme: String, // "dark" | "light" | "system"
    #[serde(default = "default_language")]
    pub language: String, // "en" | "es" | "pt" | "fr" | "de"
    pub currency: String, // "EUR" | "GBP" | "USD"
    pub default_match_mode: String, // "live" | "spectator" | "delegate"
    pub auto_save: bool,
    pub match_speed: String, // "slow" | "normal" | "fast"
    pub show_match_commentary: bool,
    pub confirm_advance: bool,
    #[serde(default = "default_ui_scale")]
    pub ui_scale: String, // "xsmall" | "small" | "normal" | "large" | "xlarge"
    #[serde(default)]
    pub high_contrast: bool,
    #[serde(default = "default_lol_hybrid_open_trade_confidence_high")]
    pub lol_hybrid_open_trade_confidence_high: f64,
    #[serde(default = "default_lol_hybrid_disengage_confidence_low")]
    pub lol_hybrid_disengage_confidence_low: f64,
}

fn default_language() -> String {
    "es".to_string()
}
fn default_ui_scale() -> String {
    "normal".to_string()
}
fn default_lol_hybrid_open_trade_confidence_high() -> f64 {
    0.60
}
fn default_lol_hybrid_disengage_confidence_low() -> f64 {
    0.32
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: "dark".to_string(),
            language: "es".to_string(),
            currency: "EUR".to_string(),
            default_match_mode: "live".to_string(),
            auto_save: true,
            match_speed: "normal".to_string(),
            show_match_commentary: true,
            confirm_advance: false,
            ui_scale: "normal".to_string(),
            high_contrast: false,
            lol_hybrid_open_trade_confidence_high: default_lol_hybrid_open_trade_confidence_high(),
            lol_hybrid_disengage_confidence_low: default_lol_hybrid_disengage_confidence_low(),
        }
    }
}

fn settings_path(app_handle: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    let dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("settings.json"))
}

#[tauri::command]
pub fn get_settings(app_handle: tauri::AppHandle) -> Result<AppSettings, String> {
    log::debug!("[cmd] get_settings");
    let path = settings_path(&app_handle)?;
    if !path.exists() {
        return Ok(AppSettings::default());
    }
    let json = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&json).map_err(|e| format!("Failed to parse settings: {}", e))
}

#[tauri::command]
pub fn save_settings(app_handle: tauri::AppHandle, settings: AppSettings) -> Result<(), String> {
    info!(
        "[cmd] save_settings: theme={}, lang={}",
        settings.theme, settings.language
    );
    let path = settings_path(&app_handle)?;
    let json = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| format!("Failed to save settings: {}", e))
}

#[tauri::command]
pub fn clear_all_saves(sm_state: tauri::State<crate::SaveManagerState>) -> Result<(), String> {
    log::warn!("[cmd] clear_all_saves: deleting all save data!");
    let mut sm = sm_state
        .0
        .lock()
        .map_err(|e| format!("Lock error: {}", e))?;
    let save_ids: Vec<String> = sm.list_saves().iter().map(|s| s.id.clone()).collect();
    for id in save_ids {
        sm.delete_save(&id)?;
    }
    Ok(())
}
