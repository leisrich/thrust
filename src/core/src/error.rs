//! Error types for the protocol translator

use thiserror::Error;

pub type Result<T> = std::result::Result<T, TranslatorError>;

#[derive(Error, Debug)]
pub enum TranslatorError {
    #[error("HID device error: {0}")]
    HidError(#[from] hidapi::HidError),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Device not found: VID {vid:04x}, PID {pid:04x}")]
    DeviceNotFound { vid: u16, pid: u16 },
    
    #[error("Device already in use")]
    DeviceInUse,
    
    #[error("Invalid HID report: {reason}")]
    InvalidReport { reason: String },
    
    #[error("FFB translation error: {reason}")]
    FfbError { reason: String },
    
    #[error("Configuration error: {reason}")]
    ConfigError { reason: String },
    
    #[error("Virtual device creation failed: {reason}")]
    VirtualDeviceError { reason: String },
    
    #[error("Calibration error: {reason}")]
    CalibrationError { reason: String },
    
    #[error("Protocol error: {reason}")]
    ProtocolError { reason: String },
    
    #[error("Timeout waiting for device response")]
    Timeout,
    
    #[error("Operation cancelled")]
    Cancelled,
    
    #[error("Feature not supported on this platform")]
    UnsupportedPlatform,
}

impl TranslatorError {
    pub fn invalid_report(reason: impl Into<String>) -> Self {
        Self::InvalidReport { reason: reason.into() }
    }
    
    pub fn ffb_error(reason: impl Into<String>) -> Self {
        Self::FfbError { reason: reason.into() }
    }
    
    pub fn config_error(reason: impl Into<String>) -> Self {
        Self::ConfigError { reason: reason.into() }
    }
    
    pub fn virtual_device_error(reason: impl Into<String>) -> Self {
        Self::VirtualDeviceError { reason: reason.into() }
    }
    
    pub fn calibration_error(reason: impl Into<String>) -> Self {
        Self::CalibrationError { reason: reason.into() }
    }
    
    pub fn protocol_error(reason: impl Into<String>) -> Self {
        Self::ProtocolError { reason: reason.into() }
    }
} 