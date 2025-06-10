//! Device communication module

pub mod thrustmaster;
pub mod virtual_g29;
pub mod descriptors;

pub use thrustmaster::ThrustmasterDevice;
pub use virtual_g29::VirtualG29Device;
pub use descriptors::{G29_HID_DESCRIPTOR, parse_hid_descriptor};

use serde::{Deserialize, Serialize};

/// Input report from Thrustmaster device (8 bytes typical)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ThrustmasterInputReport {
    pub steering: i16,        // Raw steering value
    pub throttle: u8,         // 0-255
    pub brake: u8,            // 0-255  
    pub clutch: u8,           // 0-255
    pub buttons: u16,         // Button bitfield
    pub dpad: u8,             // D-pad state (0-7, 8=center)
}

/// Input report for G29 device
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct G29InputReport {
    pub report_id: u8,        // Always 0x01
    pub steering: u16,        // 16-bit little endian, center = 0x8000
    pub throttle: u16,        // 10-bit value in 16-bit field  
    pub brake: u16,           // 10-bit value in 16-bit field
    pub clutch: u16,          // 10-bit value in 16-bit field
    pub buttons: u32,         // 24 buttons + D-pad
    pub unused: [u8; 4],      // Padding to match G29 report size
}

/// Output report from G29 (FFB commands)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct G29OutputReport {
    pub report_id: u8,
    pub data: Vec<u8>,
}

/// IFORCE command for Thrustmaster FFB
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IforceCommand {
    pub command_id: u8,
    pub data: Vec<u8>,
} 