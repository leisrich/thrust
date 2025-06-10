//! Configuration structures for the protocol translator

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub thrustmaster_config: ThrustmasterConfig,
    pub g29_config: G29Config,
    pub input_config: InputConfig,
    pub output_config: OutputConfig,
    pub ffb_config: FfbConfig,
    pub logging_config: LoggingConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            thrustmaster_config: ThrustmasterConfig::default(),
            g29_config: G29Config::default(),
            input_config: InputConfig::default(),
            output_config: OutputConfig::default(),
            ffb_config: FfbConfig::default(),
            logging_config: LoggingConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThrustmasterConfig {
    pub vid: u16,
    pub pid: u16,
    pub serial_number: Option<String>,
    pub exclusive_access: bool,
}

impl Default for ThrustmasterConfig {
    fn default() -> Self {
        Self {
            vid: 0x044F,  // Guillemot/Thrustmaster VID
            pid: 0x0004,  // Common Thrustmaster wheel PID
            serial_number: None,
            exclusive_access: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct G29Config {
    pub vid: u16,
    pub pid: u16,
    pub product_string: String,
    pub manufacturer_string: String,
    pub serial_number: String,
    pub use_custom_vid_pid: bool,
}

impl Default for G29Config {
    fn default() -> Self {
        Self {
            vid: 0x046D,  // Logitech VID
            pid: 0xC24F,  // G29 PID
            product_string: "G29 Driving Force Racing Wheel".to_string(),
            manufacturer_string: "Logitech".to_string(),
            serial_number: "TM2G29001".to_string(),
            use_custom_vid_pid: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputConfig {
    pub steering_range: u16,           // Degrees of rotation (270, 540, 900, etc.)
    pub steering_deadzone: f32,        // 0.0 - 1.0
    pub pedal_curves: PedalCurves,
    pub button_mapping: HashMap<u8, u8>, // Thrustmaster button -> G29 button
    pub axis_scaling: AxisScaling,
}

impl Default for InputConfig {
    fn default() -> Self {
        let mut button_mapping = HashMap::new();
        // Default 1:1 button mapping for first 14 buttons
        for i in 0..14 {
            button_mapping.insert(i, i);
        }
        
        Self {
            steering_range: 900,
            steering_deadzone: 0.02,
            pedal_curves: PedalCurves::default(),
            button_mapping,
            axis_scaling: AxisScaling::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PedalCurves {
    pub throttle_curve: CurveType,
    pub brake_curve: CurveType,
    pub clutch_curve: CurveType,
}

impl Default for PedalCurves {
    fn default() -> Self {
        Self {
            throttle_curve: CurveType::Linear,
            brake_curve: CurveType::Linear,
            clutch_curve: CurveType::Linear,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CurveType {
    Linear,
    Squared,
    Cubed,
    Custom(Vec<f32>), // Lookup table
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AxisScaling {
    pub steering_multiplier: f32,
    pub throttle_multiplier: f32,
    pub brake_multiplier: f32,
    pub clutch_multiplier: f32,
}

impl Default for AxisScaling {
    fn default() -> Self {
        Self {
            steering_multiplier: 1.0,
            throttle_multiplier: 1.0,
            brake_multiplier: 1.0,
            clutch_multiplier: 1.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    pub led_support: bool,
    pub led_brightness: f32,  // 0.0 - 1.0
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            led_support: true,
            led_brightness: 1.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FfbConfig {
    pub enabled: bool,
    pub global_gain: f32,     // 0.0 - 1.0
    pub spring_gain: f32,     // 0.0 - 1.0
    pub damper_gain: f32,     // 0.0 - 1.0
    pub friction_gain: f32,   // 0.0 - 1.0
    pub constant_gain: f32,   // 0.0 - 1.0
    pub periodic_gain: f32,   // 0.0 - 1.0
    pub ramp_gain: f32,       // 0.0 - 1.0
    pub autocenter_gain: f32, // 0.0 - 1.0
    pub max_force: f32,       // Maximum force in Newtons
    pub update_rate_hz: u32,  // FFB update frequency
}

impl Default for FfbConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            global_gain: 1.0,
            spring_gain: 1.0,
            damper_gain: 1.0,
            friction_gain: 1.0,
            constant_gain: 1.0,
            periodic_gain: 1.0,
            ramp_gain: 1.0,
            autocenter_gain: 0.2,
            max_force: 2.5, // Typical for consumer wheels
            update_rate_hz: 1000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub log_to_file: bool,
    pub log_file_path: Option<String>,
    pub log_hid_reports: bool,
    pub log_ffb_commands: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            log_to_file: false,
            log_file_path: None,
            log_hid_reports: false,
            log_ffb_commands: false,
        }
    }
}

impl Config {
    /// Load configuration from TOML file
    pub fn load_from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }
    
    /// Save configuration to TOML file
    pub fn save_to_file(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
} 