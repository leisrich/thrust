//! Thrustmaster device communication

use crate::device::{ThrustmasterInputReport, IforceCommand};
use crate::config::ThrustmasterConfig;
use crate::error::{TranslatorError, Result};
use hidapi::{HidApi, HidDevice};
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct ThrustmasterDevice {
    device: Arc<Mutex<HidDevice>>,
    config: ThrustmasterConfig,
}

impl ThrustmasterDevice {
    /// Open and initialize Thrustmaster device
    pub async fn open(config: &ThrustmasterConfig) -> Result<Self> {
        let api = HidApi::new()?;
        
        // Find the Thrustmaster device
        let device_info = api
            .device_list()
            .find(|dev| dev.vendor_id() == config.vid && dev.product_id() == config.pid)
            .ok_or_else(|| TranslatorError::DeviceNotFound { 
                vid: config.vid, 
                pid: config.pid 
            })?;

        tracing::info!(
            "Found Thrustmaster device: {:?} {:?}",
            device_info.manufacturer_string(),
            device_info.product_string()
        );

        let device = device_info.open_device(&api)?;
        
        // Set non-blocking mode for input reads
        device.set_blocking_mode(false)?;

        Ok(Self {
            device: Arc::new(Mutex::new(device)),
            config: config.clone(),
        })
    }

    /// Read input report from Thrustmaster device
    pub async fn read_input(&self) -> Result<Option<ThrustmasterInputReport>> {
        let device = self.device.lock().await;
        let mut buf = [0u8; 8]; // Typical Thrustmaster input report size

        match device.read(&mut buf) {
            Ok(0) => Ok(None), // No data available
            Ok(bytes_read) => {
                if bytes_read >= 8 {
                    Ok(Some(self.parse_input_report(&buf)))
                } else {
                    Err(TranslatorError::invalid_report(format!(
                        "Input report too short: {} bytes", bytes_read
                    )))
                }
            }
            Err(e) => Err(TranslatorError::HidError(e)),
        }
    }

    /// Send FFB command to Thrustmaster device
    pub async fn send_ffb_command(&self, command: IforceCommand) -> Result<()> {
        let device = self.device.lock().await;
        
        // Construct IFORCE packet
        let packet = self.build_iforce_packet(command)?;
        
        tracing::debug!("Sending IFORCE command: {:02x?}", packet);
        
        // Send via USB control transfer or feature report
        match device.send_feature_report(&packet) {
            Ok(_) => Ok(()),
            Err(e) => {
                tracing::warn!("Failed to send FFB command: {:?}", e);
                Err(TranslatorError::HidError(e))
            }
        }
    }

    fn parse_input_report(&self, data: &[u8]) -> ThrustmasterInputReport {
        // Parse Thrustmaster input report format
        // This is a simplified implementation - real format depends on specific wheel model
        
        let steering = i16::from_le_bytes([data[0], data[1]]);
        let throttle = data[2];
        let brake = data[3];
        let clutch = data[4];
        let buttons = u16::from_le_bytes([data[5], data[6]]);
        let dpad = data[7] & 0x0F; // Lower 4 bits

        ThrustmasterInputReport {
            steering,
            throttle,
            brake,
            clutch,
            buttons,
            dpad,
        }
    }

    fn build_iforce_packet(&self, command: IforceCommand) -> Result<Vec<u8>> {
        // Build IFORCE packet format
        // IFORCE packets typically have: [length, command_id, data..., checksum]
        
        let mut packet = Vec::new();
        packet.push((command.data.len() + 2) as u8); // Length including command_id and checksum
        packet.push(command.command_id);
        packet.extend_from_slice(&command.data);
        
        // Calculate checksum (XOR of all bytes except checksum itself)
        let checksum = packet.iter().fold(0u8, |acc, &byte| acc ^ byte);
        packet.push(checksum);
        
        Ok(packet)
    }

    /// Initialize wheel (set range, autocenter, etc.)
    pub async fn initialize(&self) -> Result<()> {
        // Send initialization commands
        let commands = vec![
            // Set wheel range to configured value
            IforceCommand {
                command_id: 0x01, // Set range command
                data: vec![
                    (self.config.vid & 0xFF) as u8, // Placeholder for range setting
                    (self.config.vid >> 8) as u8,
                ],
            },
            // Enable autocenter
            IforceCommand {
                command_id: 0x02, // Autocenter command
                data: vec![0x01], // Enable
            },
        ];

        for command in commands {
            self.send_ffb_command(command).await?;
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }

        tracing::info!("Thrustmaster device initialized");
        Ok(())
    }
} 