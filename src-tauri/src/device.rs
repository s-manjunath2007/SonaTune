use cpal::traits::{DeviceTrait, HostTrait};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioDeviceInfo {
    pub id: String,
    pub name: String,
    pub is_default: bool,
    pub max_channels: u16,
    pub default_sample_rate: u32,
    pub supported_sample_rates: Vec<u32>,
    pub is_output: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceStatus {
    pub connected: bool,
    pub active_device_name: String,
    pub sample_rate: u32,
    pub channels: u16,
    pub latency_ms: f32,
    pub error: Option<String>,
}

/// Enumerate real audio output devices via CPAL
pub fn get_audio_output_devices() -> Result<Vec<AudioDeviceInfo>, String> {
    let host = cpal::default_host();
    let default_output_name = host
        .default_output_device()
        .and_then(|d| d.name().ok());

    let mut device_list = Vec::new();

    let devices = host.output_devices().map_err(|e| format!("Failed to query output devices: {e}"))?;

    for device in devices {
        if let Ok(name) = device.name() {
            let is_default = default_output_name.as_deref() == Some(&name);

            let (max_channels, default_sr, supported_srs) = match device.default_output_config() {
                Ok(cfg) => {
                    let mut srs = Vec::new();
                    if let Ok(configs) = device.supported_output_configs() {
                        for c in configs {
                            let min = c.min_sample_rate().0;
                            let max = c.max_sample_rate().0;
                            for candidate in [44100, 48000, 88200, 96000, 192000] {
                                if candidate >= min && candidate <= max && !srs.contains(&candidate) {
                                    srs.push(candidate);
                                }
                            }
                        }
                    }
                    if srs.is_empty() {
                        srs.push(cfg.sample_rate().0);
                    }
                    (cfg.channels(), cfg.sample_rate().0, srs)
                }
                Err(_) => (2, 48000, vec![44100, 48000]),
            };

            device_list.push(AudioDeviceInfo {
                id: name.clone(),
                name,
                is_default,
                max_channels,
                default_sample_rate: default_sr,
                supported_sample_rates: supported_srs,
                is_output: true,
            });
        }
    }

    Ok(device_list)
}

/// Get current default audio output device info
pub fn get_default_output_device() -> Result<Option<AudioDeviceInfo>, String> {
    let devices = get_audio_output_devices()?;
    Ok(devices.into_iter().find(|d| d.is_default))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enumerate_devices() {
        let devs = get_audio_output_devices().expect("Failed to query output devices");
        println!("Found {} output devices:", devs.len());
        for d in &devs {
            println!("  - {} (default: {})", d.name, d.is_default);
        }
        assert!(!devs.is_empty(), "Expected at least one audio output device on Windows");
    }
}
