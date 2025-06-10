//! Core protocol translation library for Thrustmaster to Logitech G29
//! 
//! This crate provides the core functionality to translate between Thrustmaster
//! wheel protocols and Logitech G29 protocols, including input mapping and
//! force feedback translation.

pub mod device;
pub mod protocol;
pub mod ffb;
pub mod config;
pub mod error;

pub use device::{ThrustmasterDevice, VirtualG29Device};
pub use protocol::{InputTranslator, OutputTranslator};
pub use ffb::{FfbEngine, FfbEffect};
pub use config::Config;
pub use error::{TranslatorError, Result};

/// Main translator struct that orchestrates the protocol translation
pub struct ProtocolTranslator {
    thrustmaster: ThrustmasterDevice,
    virtual_g29: VirtualG29Device,
    input_translator: InputTranslator,
    output_translator: OutputTranslator,
    ffb_engine: FfbEngine,
    config: Config,
}

impl ProtocolTranslator {
    /// Create a new protocol translator instance
    pub async fn new(config: Config) -> Result<Self> {
        let thrustmaster = ThrustmasterDevice::open(&config.thrustmaster_config).await?;
        let virtual_g29 = VirtualG29Device::create(&config.g29_config).await?;
        let input_translator = InputTranslator::new(&config.input_config);
        let output_translator = OutputTranslator::new(&config.output_config);
        let ffb_engine = FfbEngine::new(&config.ffb_config);

        Ok(Self {
            thrustmaster,
            virtual_g29,
            input_translator,
            output_translator,
            ffb_engine,
            config,
        })
    }

    /// Start the translation loop
    pub async fn run(mut self) -> Result<()> {
        tracing::info!("Starting protocol translator");
        
        // Use Arc and Mutex to share state between tasks
        use std::sync::Arc;
        use tokio::sync::Mutex;
        
        let translator = Arc::new(Mutex::new(self));
        let translator_input = translator.clone();
        let translator_output = translator.clone();
        
        // Spawn input translation task
        let input_task = tokio::spawn(async move {
            Self::run_input_translation_task(translator_input).await
        });
        
        // Spawn output translation task  
        let output_task = tokio::spawn(async move {
            Self::run_output_translation_task(translator_output).await
        });
        
        // Run both tasks concurrently
        let (input_result, output_result) = tokio::join!(input_task, output_task);
        input_result.map_err(|e| TranslatorError::protocol_error(format!("Input task failed: {}", e)))??;
        output_result.map_err(|e| TranslatorError::protocol_error(format!("Output task failed: {}", e)))??;
        
        Ok(())
    }

    /// Handle input translation (Thrustmaster -> G29)
    async fn run_input_translation_task(translator: std::sync::Arc<tokio::sync::Mutex<Self>>) -> Result<()> {
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(1));
        
        loop {
            interval.tick().await;
            
            let mut t = translator.lock().await;
            
            // Read from Thrustmaster device
            if let Some(input_report) = t.thrustmaster.read_input().await? {
                // Translate to G29 format
                let g29_report = t.input_translator.translate(input_report);
                
                // Send to virtual G29 device
                t.virtual_g29.send_input(g29_report).await?;
            }
        }
    }

    /// Handle output translation (G29 -> Thrustmaster)
    async fn run_output_translation_task(translator: std::sync::Arc<tokio::sync::Mutex<Self>>) -> Result<()> {
        loop {
            let mut t = translator.lock().await;
            
            // Read output reports from virtual G29 device
            if let Some(output_report) = t.virtual_g29.read_output().await? {
                // Handle FFB effects
                if let Some(ffb_effect) = t.output_translator.parse_ffb_effect(output_report)? {
                    // Translate to Thrustmaster IFORCE format
                    let iforce_commands = t.ffb_engine.translate_effect(ffb_effect)?;
                    
                    // Send to Thrustmaster device
                    for command in iforce_commands {
                        t.thrustmaster.send_ffb_command(command).await?;
                    }
                }
            }
        }
    }
} 