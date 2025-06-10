//! Protocol translation between Thrustmaster and G29 formats

use crate::device::{ThrustmasterInputReport, G29InputReport, G29OutputReport};
use crate::config::{InputConfig, OutputConfig, CurveType};
use crate::ffb::FfbEffect;
use crate::error::{TranslatorError, Result};
// use std::collections::HashMap;

/// Handles input translation from Thrustmaster to G29 format
pub struct InputTranslator {
    config: InputConfig,
    last_steering: i16,
}

impl InputTranslator {
    pub fn new(config: &InputConfig) -> Self {
        Self {
            config: config.clone(),
            last_steering: 0,
        }
    }

    /// Translate Thrustmaster input report to G29 format
    pub fn translate(&mut self, input: ThrustmasterInputReport) -> G29InputReport {
        // Apply steering deadzone and scaling
        let steering = self.process_steering(input.steering);
        
        // Apply pedal curves and scaling
        let throttle = self.apply_pedal_curve(input.throttle, &self.config.pedal_curves.throttle_curve);
        let brake = self.apply_pedal_curve(input.brake, &self.config.pedal_curves.brake_curve);
        let clutch = self.apply_pedal_curve(input.clutch, &self.config.pedal_curves.clutch_curve);
        
        // Map buttons
        let buttons = self.map_buttons(input.buttons);
        
        // Include D-pad in button field (G29 style)
        let buttons_with_dpad = self.include_dpad(buttons, input.dpad);

        G29InputReport {
            report_id: 0x01,
            steering: steering as u16,
            throttle: throttle as u16,
            brake: brake as u16,
            clutch: clutch as u16,
            buttons: buttons_with_dpad,
            unused: [0; 4],
        }
    }

    fn process_steering(&mut self, raw_steering: i16) -> i16 {
        // Apply deadzone
        let normalized = raw_steering as f32 / 32767.0;
        
        let processed = if normalized.abs() < self.config.steering_deadzone {
            0.0
        } else {
            // Remove deadzone and rescale
            if normalized > 0.0 {
                (normalized - self.config.steering_deadzone) / (1.0 - self.config.steering_deadzone)
            } else {
                (normalized + self.config.steering_deadzone) / (1.0 - self.config.steering_deadzone)
            }
        };

        // Apply scaling and convert to G29 format (center = 0x8000)
        let scaled = processed * self.config.axis_scaling.steering_multiplier;
        let g29_value = (scaled * 32767.0) as i16;
        
        // G29 uses 0x8000 as center, so offset by 32768
        let result = (g29_value as i32 + 32768).clamp(0, 65535) as i16;
        
        self.last_steering = result;
        result
    }

    fn apply_pedal_curve(&self, raw_value: u8, curve: &CurveType) -> u32 {
        let normalized = raw_value as f32 / 255.0;
        
        let curved = match curve {
            CurveType::Linear => normalized,
            CurveType::Squared => normalized * normalized,
            CurveType::Cubed => normalized * normalized * normalized,
            CurveType::Custom(table) => {
                // Linear interpolation in lookup table
                let index = (normalized * (table.len() - 1) as f32) as usize;
                if index >= table.len() - 1 {
                    table[table.len() - 1]
                } else {
                    let frac = normalized * (table.len() - 1) as f32 - index as f32;
                    table[index] * (1.0 - frac) + table[index + 1] * frac
                }
            }
        };

        // G29 uses 10-bit resolution for pedals (0-1023)
        (curved * 1023.0) as u32
    }

    fn map_buttons(&self, buttons: u16) -> u32 {
        let mut mapped = 0u32;
        
        for (&thrustmaster_btn, &g29_btn) in &self.config.button_mapping {
            if buttons & (1 << thrustmaster_btn) != 0 {
                mapped |= 1 << g29_btn;
            }
        }
        
        mapped
    }

    fn include_dpad(&self, buttons: u32, dpad: u8) -> u32 {
        // G29 D-pad is encoded in the upper bits of the button field
        let dpad_value = if dpad < 8 { dpad } else { 8 }; // 8 = center
        buttons | ((dpad_value as u32) << 24)
    }
}

/// Handles output translation from G29 to Thrustmaster IFORCE format
pub struct OutputTranslator {
    config: OutputConfig,
}

impl OutputTranslator {
    pub fn new(config: &OutputConfig) -> Self {
        Self {
            config: config.clone(),
        }
    }

    /// Parse G29 output report and extract FFB effect if present
    pub fn parse_ffb_effect(&self, output: G29OutputReport) -> Result<Option<FfbEffect>> {
        if output.report_id != 0x01 || output.data.is_empty() {
            return Ok(None);
        }

        // Parse PID Device Control report (simplified)
        match output.data[0] {
            // Effect Block Index
            effect_id if effect_id > 0 && effect_id <= 40 => {
                if output.data.len() < 8 {
                    return Err(TranslatorError::invalid_report("FFB report too short"));
                }

                let effect_type = output.data[1];
                let effect = self.parse_effect_by_type(effect_id, effect_type, &output.data[2..])?;
                Ok(Some(effect))
            }
            _ => Ok(None),
        }
    }

    fn parse_effect_by_type(&self, effect_id: u8, effect_type: u8, data: &[u8]) -> Result<FfbEffect> {
        use crate::ffb::{FfbEffect, EffectType, ConstantEffect, PeriodicEffect, ConditionEffect};

        match effect_type {
            0x01 => { // Constant Force
                if data.len() < 4 {
                    return Err(TranslatorError::invalid_report("Constant effect data too short"));
                }
                
                let magnitude = i16::from_le_bytes([data[0], data[1]]);
                let duration = u16::from_le_bytes([data[2], data[3]]);
                
                Ok(FfbEffect {
                    id: effect_id,
                    effect_type: EffectType::Constant(ConstantEffect {
                        magnitude,
                        duration,
                    }),
                    gain: 255, // Will be adjusted by FFB engine
                })
            }
            0x03..=0x07 => { // Periodic effects (Square, Sine, Triangle, etc.)
                if data.len() < 6 {
                    return Err(TranslatorError::invalid_report("Periodic effect data too short"));
                }

                let magnitude = u16::from_le_bytes([data[0], data[1]]);
                let period = u16::from_le_bytes([data[2], data[3]]);
                let phase = u16::from_le_bytes([data[4], data[5]]);

                Ok(FfbEffect {
                    id: effect_id,
                    effect_type: EffectType::Periodic(PeriodicEffect {
                        magnitude,
                        period,
                        phase,
                        waveform: match effect_type {
                            0x03 => crate::ffb::Waveform::Square,
                            0x04 => crate::ffb::Waveform::Sine,
                            0x05 => crate::ffb::Waveform::Triangle,
                            0x06 => crate::ffb::Waveform::SawtoothUp,
                            0x07 => crate::ffb::Waveform::SawtoothDown,
                            _ => crate::ffb::Waveform::Sine,
                        },
                    }),
                    gain: 255,
                })
            }
            0x08..=0x0B => { // Condition effects (Spring, Damper, Inertia, Friction)
                if data.len() < 4 {
                    return Err(TranslatorError::invalid_report("Condition effect data too short"));
                }

                let positive_coefficient = i16::from_le_bytes([data[0], data[1]]);
                let negative_coefficient = i16::from_le_bytes([data[2], data[3]]);

                Ok(FfbEffect {
                    id: effect_id,
                    effect_type: EffectType::Condition(ConditionEffect {
                        positive_coefficient,
                        negative_coefficient,
                        condition_type: match effect_type {
                            0x08 => crate::ffb::ConditionType::Spring,
                            0x09 => crate::ffb::ConditionType::Damper,
                            0x0A => crate::ffb::ConditionType::Inertia,
                            0x0B => crate::ffb::ConditionType::Friction,
                            _ => crate::ffb::ConditionType::Spring,
                        },
                    }),
                    gain: 255,
                })
            }
            _ => {
                Err(TranslatorError::ffb_error(format!("Unsupported effect type: {}", effect_type)))
            }
        }
    }
} 