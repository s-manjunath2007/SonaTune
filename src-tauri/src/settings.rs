use crate::dsp::DspConfig;
use crate::presets::{get_builtin_presets, Preset};
use directories::{ProjectDirs, UserDirs};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSettings {
    pub theme: String, // "light", "dark", "system"
    pub last_page: String,
    pub active_preset_id: String,
    pub current_config: DspConfig,
    pub custom_presets: Vec<Preset>,
    pub selected_device_id: Option<String>,
    pub device_profiles: HashMap<String, DspConfig>,
    pub last_export_dir: Option<String>,
    pub minimize_to_tray: bool,
    pub start_with_windows: bool,
    pub a_b_loudness_matched: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        let presets = get_builtin_presets();
        let default_config = presets
            .first()
            .map(|p| p.config.clone())
            .unwrap_or_default();

        let default_export_dir = UserDirs::new()
            .and_then(|u| {
                u.audio_dir()
                    .or(u.document_dir())
                    .or(u.desktop_dir())
                    .map(|p| p.to_string_lossy().to_string())
            });

        Self {
            theme: "light".to_string(),
            last_page: "sound".to_string(),
            active_preset_id: "balanced".to_string(),
            current_config: default_config,
            custom_presets: Vec::new(),
            selected_device_id: None,
            device_profiles: HashMap::new(),
            last_export_dir: default_export_dir,
            minimize_to_tray: true,
            start_with_windows: false,
            a_b_loudness_matched: true,
        }
    }
}

pub struct SettingsManager {
    config_path: PathBuf,
}

impl SettingsManager {
    pub fn new() -> Self {
        let path = if let Some(proj_dirs) = ProjectDirs::from("com", "sonatune", "SonaTune") {
            let config_dir = proj_dirs.config_dir();
            let _ = fs::create_dir_all(config_dir);
            config_dir.join("settings.json")
        } else {
            PathBuf::from("settings.json")
        };

        Self { config_path: path }
    }

    pub fn load(&self) -> AppSettings {
        if self.config_path.exists() {
            match fs::read_to_string(&self.config_path) {
                Ok(content) => match serde_json::from_str::<AppSettings>(&content) {
                    Ok(settings) => return settings,
                    Err(e) => {
                        let timestamp = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_secs())
                            .unwrap_or(0);
                        let backup_path = self.config_path.with_file_name(format!("settings.json.bak.{timestamp}"));
                        let _ = fs::copy(&self.config_path, &backup_path);
                        eprintln!("Corrupted settings.json detected ({e}). Backed up to {:?} and resetting to defaults.", backup_path);
                    }
                },
                Err(e) => {
                    eprintln!("Failed to read settings file {:?}: {e}", self.config_path);
                }
            }
        }
        let default_settings = AppSettings::default();
        let _ = self.save(&default_settings);
        default_settings
    }

    pub fn save(&self, settings: &AppSettings) -> Result<(), String> {
        if let Some(parent) = self.config_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let json = serde_json::to_string_pretty(settings)
            .map_err(|e| format!("Serialization error: {e}"))?;
        fs::write(&self.config_path, json).map_err(|e| format!("Failed to write settings file: {e}"))?;
        Ok(())
    }

    pub fn reset(&self) -> Result<AppSettings, String> {
        let defaults = AppSettings::default();
        self.save(&defaults)?;
        Ok(defaults)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_backwards_compatible_deserialization() {
        // Test parsing JSON missing fields from future/older releases
        let partial_json = r#"{"theme": "dark", "active_preset_id": "vocal"}"#;
        let parsed: AppSettings = serde_json::from_str(partial_json).expect("Partial JSON must deserialize using defaults");
        assert_eq!(parsed.theme, "dark");
        assert_eq!(parsed.active_preset_id, "vocal");
        assert_eq!(parsed.minimize_to_tray, true, "Missing field should be populated from default");
        assert!(parsed.a_b_loudness_matched);
    }

    #[test]
    fn test_settings_corruption_creates_backup() {
        let temp_dir = std::env::temp_dir().join(format!("sonatune_test_settings_{}", std::process::id()));
        let _ = fs::create_dir_all(&temp_dir);
        let config_file = temp_dir.join("settings.json");

        // Write corrupt JSON
        fs::write(&config_file, "{ corrupted JSON syntax !!! ").unwrap();

        let manager = SettingsManager { config_path: config_file.clone() };
        let loaded = manager.load();

        // Should load default settings cleanly
        assert_eq!(loaded.theme, "light");

        // A backup file should have been created in the same directory
        let mut found_backup = false;
        if let Ok(entries) = fs::read_dir(&temp_dir) {
            for entry in entries.flatten() {
                if entry.file_name().to_string_lossy().contains("settings.json.bak") {
                    found_backup = true;
                    break;
                }
            }
        }
        assert!(found_backup, "Corrupted settings file must be backed up before reset");

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
