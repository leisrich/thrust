# Thrustmaster to G29 Protocol Translator

A high-performance protocol translator that makes Thrustmaster/Guillemot Force-Feedback racing wheels appear to a PC as a Logitech G29, enabling native support in games and simulators without requiring game-specific configurations.

## Features

- **Universal Game Support**: Works with any game or simulator that supports the Logitech G29
- **Full Force Feedback**: Translates all FFB effects (constant, spring, damper, periodic, ramp)
- **Low Latency**: < 2ms translation latency with 1000Hz update rate
- **Cross-Platform**: Windows 10+, Linux 6.x, macOS 13+
- **Configurable**: Extensive customization for button mapping, pedal curves, and FFB settings
- **LED Support**: Rev-strip LED translation (when supported by hardware)

## Supported Hardware

### Thrustmaster Wheels
- VID: `0x044F` (Guillemot/Thrustmaster)
- PID: `0x0004` and other Thrustmaster wheel PIDs
- Any wheel supported by the Linux `iforce` driver

### Target Device
- Logitech G29 (VID: `0x046D`, PID: `0xC24F`)
- Complete HID descriptor and PID FFB compatibility

## Quick Start

### Installation

#### From Releases (Recommended)
Download the latest release for your platform from the [Releases](../../releases) page.

#### From Source
```bash
git clone https://github.com/your-org/thrustmaster-g29-translator
cd thrustmaster-g29-translator
cargo build --release
```

### Basic Usage

1. **Connect your Thrustmaster wheel** (ensure it's detected by the system)

2. **Generate configuration file:**
   ```bash
   ./target/release/tm-g29 config
   ```

3. **Discover devices:**
   ```bash
   ./target/release/tm-g29 discover --detailed
   ```

4. **Calibrate wheel (optional but recommended):**
   ```bash
   ./target/release/tm-g29 calibrate
   ```

5. **Run the translator:**
   ```bash
   ./target/release/tm-g29 run
   ```

6. **Test with a game!** Your Thrustmaster wheel should now appear as a G29 in game controller settings.

## Platform-Specific Setup

### Windows

1. **Install ViGEm Bus Driver:**
   ```powershell
   # Download and install from: https://github.com/ViGEm/ViGEmBus/releases
   ```

2. **Optional - Install HidGuardian** (for exclusive access):
   ```powershell
   # Download from: https://github.com/ViGEm/HidGuardian/releases
   ```

3. **Run as Administrator** (required for device creation)

### Linux

1. **Install required permissions:**
   ```bash
   sudo usermod -a -G input $USER
   sudo udevadm control --reload-rules
   ```

2. **Create udev rule** (create `/etc/udev/rules.d/99-thrustmaster-g29.rules`):
   ```
   SUBSYSTEM=="hidraw", ATTRS{idVendor}=="044f", MODE="0666"
   SUBSYSTEM=="uinput", MODE="0666"
   ```

3. **Load required modules:**
   ```bash
   sudo modprobe uinput
   sudo modprobe hid-pidff
   ```

### macOS

1. **Install VirtualHIDDevice** (if not using system provided):
   ```bash
   # Follow instructions at: https://github.com/pqrs-org/Karabiner-VirtualHIDDevice
   ```

2. **Grant permissions** in System Preferences → Security & Privacy → Input Monitoring

## Configuration

The translator uses a TOML configuration file (`config.toml` by default). Generate a default configuration:

```bash
tm-g29 config
```

### Key Configuration Sections

#### Thrustmaster Device
```toml
[thrustmaster_config]
vid = 0x044F              # Vendor ID
pid = 0x0004              # Product ID  
exclusive_access = true   # Grab device exclusively
```

#### Input Mapping
```toml
[input_config]
steering_range = 900          # Degrees of rotation
steering_deadzone = 0.02      # Deadzone (0.0-1.0)

[input_config.axis_scaling]
steering_multiplier = 1.0
throttle_multiplier = 1.0
brake_multiplier = 1.0
clutch_multiplier = 1.0

[input_config.pedal_curves]
throttle_curve = "Linear"     # Linear, Squared, Cubed, or Custom
brake_curve = "Linear"
clutch_curve = "Linear"
```

#### Force Feedback
```toml
[ffb_config]
enabled = true
global_gain = 1.0            # Master gain (0.0-1.0)
spring_gain = 1.0            # Spring effect gain
damper_gain = 1.0            # Damper effect gain
constant_gain = 1.0          # Constant force gain
max_force = 2.5              # Maximum force in Newtons
update_rate_hz = 1000        # FFB update frequency
```

## CLI Commands

### Device Discovery
```bash
# List compatible devices
tm-g29 discover

# Detailed device information
tm-g29 discover --detailed
```

### Testing
```bash
# Test input translation (no virtual device)
tm-g29 test --duration 30

# Test FFB effects
tm-g29 ffb-test --effect constant --duration 5
tm-g29 ffb-test --effect spring --duration 10
```

### Calibration
```bash
# Full calibration
tm-g29 calibrate

# Skip steering calibration
tm-g29 calibrate --skip-steering

# Skip pedal calibration  
tm-g29 calibrate --skip-pedals
```

### Running
```bash
# Run in foreground with verbose logging
tm-g29 run --foreground -v

# Run in background (daemon mode)
tm-g29 run

# Custom config file
tm-g29 -c /path/to/config.toml run
```

## Technical Details

### Protocol Translation

The translator operates at the HID report level:

1. **Input Path**: Thrustmaster HID reports → Canonical format → G29 HID reports
2. **Output Path**: G29 PID FFB reports → Canonical effects → IFORCE commands

### Latency Optimization

- **1ms USB polling** on both input and output
- **Lock-free data structures** for inter-thread communication  
- **Pre-computed effect tables** for FFB translation
- **Dedicated real-time thread** for FFB processing

### Force Feedback Translation

| G29 Effect Type | IFORCE Command | Notes |
|----------------|----------------|-------|
| Constant Force | `0x41` | Direct magnitude mapping |
| Spring | `0x43` | Coefficient-based |
| Damper | `0x43` | Velocity-based resistance |
| Periodic (Sine/Square) | `0x42` | Waveform + frequency |
| Ramp | `0x44` | Start/end magnitude |

### Virtual Device Implementation

- **Windows**: ViGEm Bus driver with custom G29 profile
- **Linux**: uinput device with complete G29 HID descriptor
- **macOS**: VirtualHIDDevice framework with IOKit integration

## Troubleshooting

### Common Issues

#### "Device not found" error
```bash
# Check if device is detected
tm-g29 discover --detailed

# Verify VID/PID in config matches your wheel
# Check USB connection and drivers
```

#### Force feedback not working
```bash
# Test FFB directly
tm-g29 ffb-test --effect constant

# Check FFB configuration
[ffb_config]
enabled = true
global_gain = 1.0  # Try reducing if too strong
```

#### Wheel not recognized in games
```bash
# Verify virtual device creation
# On Windows: Check Device Manager for "Logitech G29"
# On Linux: Check dmesg for uinput device creation
# Restart the game after starting translator
```

#### Permission denied errors
```bash
# Linux: Check udev rules and group membership
sudo usermod -a -G input $USER
newgrp input

# Windows: Run as Administrator
# macOS: Grant Input Monitoring permissions
```

### Debug Mode

Enable verbose logging for troubleshooting:
```bash
tm-g29 run -v --log-file debug.log
```

## Development

### Building from Source

```bash
# Clone repository
git clone https://github.com/your-org/thrustmaster-g29-translator
cd thrustmaster-g29-translator

# Build all components
cargo build --release

# Run tests
cargo test

# Run with development config
cargo run --bin tm-g29 -- config
cargo run --bin tm-g29 -- run -v
```

### Project Structure

```
src/
├── core/           # Core translation library
│   ├── device/     # Device communication
│   ├── protocol/   # Protocol translation
│   ├── ffb/        # Force feedback engine
│   └── config/     # Configuration management
├── cli/            # Command-line interface
├── gui/            # GUI application (future)
└── windows/        # Windows-specific code
    linux/          # Linux-specific code
    macos/          # macOS-specific code
```

### Testing

Run the test suite:
```bash
# Unit tests
cargo test

# Integration tests with actual hardware
cargo test --test integration -- --ignored

# Property-based tests
cargo test --features proptest
```

## Contributing

1. Fork the repository
2. Create a feature branch: `git checkout -b feature-name`
3. Make your changes and add tests
4. Run the test suite: `cargo test`
5. Submit a pull request

### Code Style

- Use `cargo fmt` for formatting
- Use `cargo clippy` for linting
- Follow Rust API guidelines
- Add documentation for public APIs

## License

This project is licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.

## Acknowledgments

- [Logitech G29 HID descriptor reverse engineering](https://github.com/berarma/new-lg4ff)
- [Linux IFORCE driver documentation](https://www.kernel.org/doc/html/latest/input/ff.html)
- [ViGEm Bus driver](https://github.com/ViGEm/ViGEmBus) for Windows virtual device support
- [MiSTer IFORCE implementation](https://github.com/MiSTer-devel/MiSTer/issues/799) for protocol insights

## Disclaimer

This software is not affiliated with, endorsed by, or sponsored by Logitech or Thrustmaster. All trademarks are the property of their respective owners.

Use at your own risk. The authors are not responsible for any damage to hardware or software resulting from the use of this translator. # thrust
