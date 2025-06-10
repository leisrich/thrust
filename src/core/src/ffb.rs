//! Force Feedback translation engine

use crate::device::IforceCommand;
use crate::config::FfbConfig;
use crate::error::{TranslatorError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Main FFB engine for translating effects
pub struct FfbEngine {
    config: FfbConfig,
    active_effects: HashMap<u8, ActiveEffect>,
    last_update: Instant,
}

impl FfbEngine {
    pub fn new(config: &FfbConfig) -> Self {
        Self {
            config: config.clone(),
            active_effects: HashMap::new(),
            last_update: Instant::now(),
        }
    }

    /// Translate a G29 FFB effect to IFORCE commands
    pub fn translate_effect(&mut self, effect: FfbEffect) -> Result<Vec<IforceCommand>> {
        if !self.config.enabled {
            return Ok(vec![]);
        }

        let mut commands = Vec::new();

        // Store effect as active
        let active_effect = ActiveEffect {
            effect: effect.clone(),
            start_time: Instant::now(),
            enabled: true,
        };
        self.active_effects.insert(effect.id, active_effect);

        // Generate IFORCE commands based on effect type
        match &effect.effect_type {
            EffectType::Constant(constant) => {
                commands.extend(self.translate_constant_effect(effect.id, constant)?);
            }
            EffectType::Periodic(periodic) => {
                commands.extend(self.translate_periodic_effect(effect.id, periodic)?);
            }
            EffectType::Condition(condition) => {
                commands.extend(self.translate_condition_effect(effect.id, condition)?);
            }
            EffectType::Ramp(ramp) => {
                commands.extend(self.translate_ramp_effect(effect.id, ramp)?);
            }
        }

        Ok(commands)
    }

    /// Generate periodic update commands for active effects
    pub fn update_active_effects(&mut self) -> Result<Vec<IforceCommand>> {
        let now = Instant::now();
        if now.duration_since(self.last_update) < Duration::from_millis(1000 / self.config.update_rate_hz as u64) {
            return Ok(vec![]);
        }

        let mut commands = Vec::new();

        // Remove expired effects
        self.active_effects.retain(|_, effect| {
            if let EffectType::Constant(constant) = &effect.effect.effect_type {
                if constant.duration > 0 {
                    let elapsed = now.duration_since(effect.start_time);
                    return elapsed < Duration::from_millis(constant.duration as u64);
                }
            }
            true
        });

        // Update periodic effects
        for (effect_id, active_effect) in &self.active_effects {
            if let EffectType::Periodic(periodic) = &active_effect.effect.effect_type {
                if let Some(cmd) = self.update_periodic_effect(*effect_id, periodic, now)? {
                    commands.push(cmd);
                }
            }
        }

        self.last_update = now;
        Ok(commands)
    }

    fn translate_constant_effect(&self, effect_id: u8, effect: &ConstantEffect) -> Result<Vec<IforceCommand>> {
        let magnitude = self.apply_gain(effect.magnitude, self.config.constant_gain);
        let scaled_magnitude = self.scale_magnitude(magnitude);

        // IFORCE constant force command (simplified)
        let cmd = IforceCommand {
            command_id: 0x41, // Constant force
            data: vec![
                effect_id,
                scaled_magnitude as u8,
                (scaled_magnitude >> 8) as u8,
                (effect.duration & 0xFF) as u8,
                (effect.duration >> 8) as u8,
            ],
        };

        Ok(vec![cmd])
    }

    fn translate_periodic_effect(&self, effect_id: u8, effect: &PeriodicEffect) -> Result<Vec<IforceCommand>> {
        let magnitude = self.apply_gain(effect.magnitude as i16, self.config.periodic_gain);
        let scaled_magnitude = self.scale_magnitude(magnitude);

        // IFORCE periodic effect command
        let waveform_id = match effect.waveform {
            Waveform::Sine => 0x01,
            Waveform::Square => 0x02,
            Waveform::Triangle => 0x03,
            Waveform::SawtoothUp => 0x04,
            Waveform::SawtoothDown => 0x05,
        };

        let cmd = IforceCommand {
            command_id: 0x42, // Periodic effect
            data: vec![
                effect_id,
                waveform_id,
                scaled_magnitude as u8,
                (scaled_magnitude >> 8) as u8,
                (effect.period & 0xFF) as u8,
                (effect.period >> 8) as u8,
                (effect.phase & 0xFF) as u8,
                (effect.phase >> 8) as u8,
            ],
        };

        Ok(vec![cmd])
    }

    fn translate_condition_effect(&self, effect_id: u8, effect: &ConditionEffect) -> Result<Vec<IforceCommand>> {
        let gain = match effect.condition_type {
            ConditionType::Spring => self.config.spring_gain,
            ConditionType::Damper => self.config.damper_gain,
            ConditionType::Inertia => 1.0, // Not specifically configurable
            ConditionType::Friction => self.config.friction_gain,
        };

        let pos_coeff = self.apply_gain(effect.positive_coefficient, gain);
        let neg_coeff = self.apply_gain(effect.negative_coefficient, gain);

        let condition_id = match effect.condition_type {
            ConditionType::Spring => 0x01,
            ConditionType::Damper => 0x02,
            ConditionType::Inertia => 0x03,
            ConditionType::Friction => 0x04,
        };

        let cmd = IforceCommand {
            command_id: 0x43, // Condition effect
            data: vec![
                effect_id,
                condition_id,
                (pos_coeff & 0xFF) as u8,
                (pos_coeff >> 8) as u8,
                (neg_coeff & 0xFF) as u8,
                (neg_coeff >> 8) as u8,
            ],
        };

        Ok(vec![cmd])
    }

    fn translate_ramp_effect(&self, effect_id: u8, effect: &RampEffect) -> Result<Vec<IforceCommand>> {
        let start_magnitude = self.apply_gain(effect.start_magnitude, self.config.ramp_gain);
        let end_magnitude = self.apply_gain(effect.end_magnitude, self.config.ramp_gain);

        let cmd = IforceCommand {
            command_id: 0x44, // Ramp effect
            data: vec![
                effect_id,
                (start_magnitude & 0xFF) as u8,
                (start_magnitude >> 8) as u8,
                (end_magnitude & 0xFF) as u8,
                (end_magnitude >> 8) as u8,
                (effect.duration & 0xFF) as u8,
                (effect.duration >> 8) as u8,
            ],
        };

        Ok(vec![cmd])
    }

    fn update_periodic_effect(&self, effect_id: u8, effect: &PeriodicEffect, now: Instant) -> Result<Option<IforceCommand>> {
        // Calculate current phase based on time and period
        let elapsed = now.duration_since(self.last_update);
        let phase_increment = (elapsed.as_millis() as f32 / effect.period as f32 * 360.0) as u16;
        
        // This would normally update the effect phase, but for simplicity we'll skip
        // dynamic updates in this basic implementation
        Ok(None)
    }

    fn apply_gain(&self, value: i16, gain: f32) -> i16 {
        let adjusted = (value as f32 * gain * self.config.global_gain).clamp(-32767.0, 32767.0);
        adjusted as i16
    }

    fn scale_magnitude(&self, magnitude: i16) -> i16 {
        // Scale to IFORCE range and apply max force limit
        let force_ratio = self.config.max_force / 2.5; // Assuming 2.5N baseline
        let scaled = (magnitude as f32 * force_ratio).clamp(-32767.0, 32767.0);
        scaled as i16
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FfbEffect {
    pub id: u8,
    pub effect_type: EffectType,
    pub gain: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EffectType {
    Constant(ConstantEffect),
    Periodic(PeriodicEffect),
    Condition(ConditionEffect),
    Ramp(RampEffect),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstantEffect {
    pub magnitude: i16,
    pub duration: u16, // milliseconds, 0 = infinite
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeriodicEffect {
    pub magnitude: u16,
    pub period: u16,    // milliseconds
    pub phase: u16,     // degrees (0-359)
    pub waveform: Waveform,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Waveform {
    Sine,
    Square,
    Triangle,
    SawtoothUp,
    SawtoothDown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionEffect {
    pub positive_coefficient: i16,
    pub negative_coefficient: i16,
    pub condition_type: ConditionType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConditionType {
    Spring,
    Damper,
    Inertia,
    Friction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RampEffect {
    pub start_magnitude: i16,
    pub end_magnitude: i16,
    pub duration: u16,
}

#[derive(Debug, Clone)]
struct ActiveEffect {
    effect: FfbEffect,
    start_time: Instant,
    enabled: bool,
} 