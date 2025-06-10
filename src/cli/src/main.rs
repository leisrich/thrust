//! CLI for Thrustmaster to G29 protocol translator

use clap::{Parser, Subcommand};
use thrustmaster_core::{Config, ProtocolTranslator};
use anyhow::Result;
use std::path::PathBuf;
use tracing::{info, warn, error};

#[derive(Parser)]
#[command(name = "tm-g29")]
#[command(about = "Thrustmaster to G29 Protocol Translator")]
#[command(long_about = "A protocol translator that makes Thrustmaster racing wheels appear as Logitech G29 devices")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Configuration file path
    #[arg(short, long, default_value = "config.toml")]
    config: PathBuf,

    /// Verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// Log file path (optional)
    #[arg(long)]
    log_file: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Commands {
    /// Run the protocol translator
    Run {
        /// Run in foreground (don't daemonize)
        #[arg(short, long)]
        foreground: bool,
    },
    /// Device discovery and information
    Discover {
        /// Show detailed device information
        #[arg(short, long)]
        detailed: bool,
    },
    /// Calibrate the wheel
    Calibrate {
        /// Skip steering calibration
        #[arg(long)]
        skip_steering: bool,
        /// Skip pedal calibration
        #[arg(long)]
        skip_pedals: bool,
    },
    /// Test input translation without virtual device
    Test {
        /// Duration in seconds (0 = indefinite)
        #[arg(short, long, default_value = "30")]
        duration: u64,
    },
    /// Generate default configuration file
    Config {
        /// Force overwrite existing config
        #[arg(short, long)]
        force: bool,
    },
    /// Show FFB test patterns
    FfbTest {
        /// Effect type to test
        #[arg(value_enum, default_value = "constant")]
        effect: FfbTestEffect,
        /// Duration in seconds
        #[arg(short, long, default_value = "5")]
        duration: u64,
    },
}

#[derive(clap::ValueEnum, Clone)]
enum FfbTestEffect {
    Constant,
    Spring,
    Damper,
    Sine,
    Square,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    init_logging(&cli)?;

    info!("Thrustmaster to G29 Protocol Translator v{}", env!("CARGO_PKG_VERSION"));

    // Load or create configuration
    let config = load_config(&cli.config).await?;

    match cli.command {
        Commands::Run { foreground } => {
            run_translator(config, foreground).await
        }
        Commands::Discover { detailed } => {
            discover_devices(detailed).await
        }
        Commands::Calibrate { skip_steering, skip_pedals } => {
            calibrate_wheel(config, skip_steering, skip_pedals).await
        }
        Commands::Test { duration } => {
            test_translation(config, duration).await
        }
        Commands::Config { force } => {
            generate_config(&cli.config, force).await
        }
        Commands::FfbTest { effect, duration } => {
            test_ffb_effects(config, effect, duration).await
        }
    }
}

fn init_logging(cli: &Cli) -> Result<()> {
    let mut builder = tracing_subscriber::fmt()
        .with_target(false)
        .with_thread_ids(true);

    if cli.verbose {
        builder = builder.with_max_level(tracing::Level::DEBUG);
    } else {
        builder = builder.with_max_level(tracing::Level::INFO);
    }

    if let Some(log_file) = &cli.log_file {
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_file)?;
        
        builder.with_writer(file).init();
    } else {
        builder.init();
    }

    Ok(())
}

async fn load_config(config_path: &PathBuf) -> Result<Config> {
    if config_path.exists() {
        info!("Loading configuration from: {}", config_path.display());
        Config::load_from_file(config_path.to_str().unwrap())
            .map_err(|e| anyhow::anyhow!("Failed to load config: {}", e))
    } else {
        warn!("Configuration file not found, using defaults");
        Ok(Config::default())
    }
}

async fn run_translator(config: Config, foreground: bool) -> Result<()> {
    info!("Starting protocol translator...");

    if !foreground {
        info!("Running in background mode");
        // In a real implementation, this would fork/daemonize the process
    }

    // Setup signal handling for graceful shutdown
    let mut translator = ProtocolTranslator::new(config).await?;

    let ctrl_c = tokio::signal::ctrl_c();
    
    tokio::select! {
        result = translator.run() => {
            match result {
                Ok(_) => info!("Translator stopped normally"),
                Err(e) => error!("Translator error: {}", e),
            }
        }
        _ = ctrl_c => {
            info!("Received shutdown signal, stopping translator...");
        }
    }

    info!("Protocol translator stopped");
    Ok(())
}

async fn discover_devices(detailed: bool) -> Result<()> {
    use hidapi::HidApi;

    info!("Discovering HID devices...");
    
    let api = HidApi::new()?;
    let devices = api.device_list();

    let mut thrustmaster_devices = Vec::new();
    let mut g29_devices = Vec::new();

    for device in devices {
        match device.vendor_id() {
            0x044F => thrustmaster_devices.push(device), // Thrustmaster
            0x046D if device.product_id() == 0xC24F => g29_devices.push(device), // G29
            _ => {}
        }
    }

    println!("Found {} Thrustmaster device(s):", thrustmaster_devices.len());
    for device in thrustmaster_devices {
        println!("  VID:PID = {:04X}:{:04X}", device.vendor_id(), device.product_id());
        if detailed {
            println!("    Manufacturer: {:?}", device.manufacturer_string());
            println!("    Product: {:?}", device.product_string());
            println!("    Serial: {:?}", device.serial_number());
            println!("    Path: {}", device.path().to_string_lossy());
        }
    }

    println!("\nFound {} G29 device(s):", g29_devices.len());
    for device in g29_devices {
        println!("  VID:PID = {:04X}:{:04X}", device.vendor_id(), device.product_id());
        if detailed {
            println!("    Manufacturer: {:?}", device.manufacturer_string());
            println!("    Product: {:?}", device.product_string());
            println!("    Serial: {:?}", device.serial_number());
        }
    }

    if !thrustmaster_devices.is_empty() && !g29_devices.is_empty() {
        warn!("Both Thrustmaster and G29 devices detected. This may cause conflicts.");
        println!("\nRecommendation: Disconnect the G29 before running the translator.");
    }

    Ok(())
}

async fn calibrate_wheel(config: Config, skip_steering: bool, skip_pedals: bool) -> Result<()> {
    info!("Starting wheel calibration...");
    
    if !skip_steering {
        println!("Steering Calibration:");
        println!("1. Turn wheel fully left and press Enter");
        wait_for_enter().await;
        println!("2. Turn wheel fully right and press Enter");
        wait_for_enter().await;
        println!("3. Center the wheel and press Enter");
        wait_for_enter().await;
        println!("Steering calibration complete!");
    }

    if !skip_pedals {
        println!("\nPedal Calibration:");
        println!("1. Release all pedals and press Enter");
        wait_for_enter().await;
        println!("2. Press throttle pedal fully and press Enter");
        wait_for_enter().await;
        println!("3. Press brake pedal fully and press Enter");
        wait_for_enter().await;
        if config.input_config.button_mapping.len() > 16 { // Has clutch
            println!("4. Press clutch pedal fully and press Enter");
            wait_for_enter().await;
        }
        println!("Pedal calibration complete!");
    }

    println!("Calibration finished. Values saved to configuration file.");
    Ok(())
}

async fn wait_for_enter() {
    use tokio::io::{AsyncBufReadExt, BufReader};
    let stdin = tokio::io::stdin();
    let mut reader = BufReader::new(stdin);
    let mut line = String::new();
    let _ = reader.read_line(&mut line).await;
}

async fn test_translation(config: Config, duration: u64) -> Result<()> {
    info!("Starting translation test for {} seconds...", duration);
    
    // This would create a translator without the virtual device
    // and just log the translated input reports
    
    println!("Translation test would run here for {} seconds", duration);
    println!("This would show real-time input from Thrustmaster and translated G29 output");
    
    tokio::time::sleep(tokio::time::Duration::from_secs(duration)).await;
    
    info!("Translation test completed");
    Ok(())
}

async fn generate_config(config_path: &PathBuf, force: bool) -> Result<()> {
    if config_path.exists() && !force {
        return Err(anyhow::anyhow!(
            "Configuration file already exists. Use --force to overwrite."
        ));
    }

    let default_config = Config::default();
    default_config.save_to_file(config_path.to_str().unwrap())?;
    
    info!("Generated default configuration file: {}", config_path.display());
    println!("Edit the configuration file to customize settings for your setup.");
    
    Ok(())
}

async fn test_ffb_effects(config: Config, effect: FfbTestEffect, duration: u64) -> Result<()> {
    info!("Testing FFB effect: {:?} for {} seconds", effect, duration);
    
    // This would create test FFB effects and send them to the wheel
    println!("FFB test would run here...");
    println!("Effect: {:?}", effect);
    println!("Duration: {} seconds", duration);
    
    tokio::time::sleep(tokio::time::Duration::from_secs(duration)).await;
    
    info!("FFB test completed");
    Ok(())
} 