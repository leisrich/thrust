# Architecture Overview

## System Architecture

The Thrustmaster to G29 Protocol Translator follows a modular, layered architecture designed for low-latency real-time translation between HID protocols.

```
┌─────────────────┐    ┌─────────────────────────┐    ┌─────────────────┐
│  Game/Simulator │    │     Operating System    │    │ Thrustmaster    │
│                 │    │                         │    │     Wheel       │
└─────────────────┘    └─────────────────────────┘    └─────────────────┘
         │                          │                           │
         │ G29 HID Reports          │ Virtual Device            │ Physical HID
         ▼                          ▼                           ▼
┌─────────────────┐    ┌─────────────────────────┐    ┌─────────────────┐
│ Virtual G29     │◄───┤   Protocol Translator   ├───►│ Physical Device │
│ Device          │    │        (Core)           │    │   Interface     │
└─────────────────┘    └─────────────────────────┘    └─────────────────┘
```

## Core Components

### 1. Protocol Translator Core (`src/core/`)

The heart of the system, responsible for:
- Device lifecycle management
- Real-time protocol translation
- Force feedback effect translation
- Configuration management

#### Key Modules:

- **`lib.rs`**: Main orchestrator, manages the translation loop
- **`device/`**: Device abstraction and communication
- **`protocol/`**: Input/output protocol translation
- **`ffb/`**: Force feedback effect translation engine
- **`config/`**: Configuration management
- **`error/`**: Error handling and types

### 2. Device Layer (`src/core/src/device/`)

Handles low-level device communication:

```rust
pub trait DeviceInterface {
    async fn read_input(&self) -> Result<Option<InputReport>>;
    async fn send_output(&self, report: OutputReport) -> Result<()>;
}
```

#### Thrustmaster Device (`thrustmaster.rs`)
- HID communication via `hidapi`
- IFORCE protocol implementation
- Exclusive device access management
- Input report parsing (wheel-specific formats)

#### Virtual G29 Device (`virtual_g29.rs`)
- Platform-specific virtual device creation
- G29 HID descriptor implementation
- Output report interception (FFB commands from games)

#### Platform Implementations:
- **Windows**: ViGEm Bus driver integration
- **Linux**: uinput device with complete HID descriptors
- **macOS**: VirtualHIDDevice framework

### 3. Protocol Translation (`src/core/src/protocol.rs`)

Bidirectional translation between protocols:

#### Input Translation (Thrustmaster → G29)
```rust
struct InputTranslator {
    fn translate(&mut self, input: ThrustmasterInputReport) -> G29InputReport
}
```

**Translation Pipeline:**
1. Raw Thrustmaster HID report
2. Parse device-specific format
3. Apply calibration and curves
4. Map buttons and axes
5. Generate G29-compatible report

#### Output Translation (G29 → Thrustmaster)
```rust  
struct OutputTranslator {
    fn parse_ffb_effect(&self, output: G29OutputReport) -> Result<Option<FfbEffect>>
}
```

**Translation Pipeline:**
1. Intercept G29 PID FFB reports
2. Parse effect parameters
3. Translate to canonical effect format
4. Convert to IFORCE commands

### 4. Force Feedback Engine (`src/core/src/ffb.rs`)

Real-time FFB effect processing:

```rust
struct FfbEngine {
    fn translate_effect(&mut self, effect: FfbEffect) -> Result<Vec<IforceCommand>>;
    fn update_active_effects(&mut self) -> Result<Vec<IforceCommand>>;
}
```

#### Supported Effects:
- **Constant Force**: Direct magnitude mapping
- **Spring**: Position-based resistance  
- **Damper**: Velocity-based resistance
- **Friction**: Surface texture simulation
- **Periodic**: Sine, square, triangle waves
- **Ramp**: Time-varying force

#### Effect Translation Table:

| G29 PID Effect | IFORCE Command | Parameters |
|---------------|----------------|------------|
| ET_CONSTANT   | 0x41          | magnitude, duration |
| ET_SPRING     | 0x43 (type 1) | pos_coeff, neg_coeff |
| ET_DAMPER     | 0x43 (type 2) | pos_coeff, neg_coeff |
| ET_SINE       | 0x42 (wave 1) | magnitude, period, phase |
| ET_SQUARE     | 0x42 (wave 2) | magnitude, period, phase |
| ET_RAMP       | 0x44          | start_mag, end_mag, duration |

## Data Flow

### Input Path (1ms loop)
```
Thrustmaster HID → Raw Report → Parse → Translate → G29 Report → Virtual Device
    ↑                                                                    ↓
USB Poll (1kHz)                                                   OS Game Input
```

### Output Path (Event-driven)
```
Game FFB Command → Virtual Device → Parse PID → Translate → IFORCE → Thrustmaster
                                      ↓                        ↓
                                 Effect Queue              USB Control Transfer
```

## Threading Model

### Main Thread
- Device initialization
- Configuration management  
- Signal handling
- Graceful shutdown

### Input Thread (High Priority)
- 1ms polling loop
- Input report processing
- Translation pipeline
- Virtual device updates

### Output Thread (Real-time)
- FFB command processing
- Effect queue management
- IFORCE packet generation
- Hardware communication

### Effect Update Thread
- Periodic effect updates (1kHz)
- Effect lifecycle management
- Timing-critical operations

## Memory Management

### Zero-Copy Design
- Direct buffer manipulation where possible
- Minimize allocations in hot paths
- Pre-allocated effect tables

### Lock-Free Communication
- SPSC queues for thread communication
- Atomic operations for state updates
- Lock-free data structures for real-time paths

## Configuration System

### Hierarchical Configuration
```toml
[thrustmaster_config]
vid = 0x044F
pid = 0x0004

[input_config]
steering_range = 900
[input_config.button_mapping]
0 = 2  # X → B button

[ffb_config]
global_gain = 1.0
spring_gain = 1.2
```

### Runtime Reconfiguration
- Hot-reload of non-critical settings
- Validation and error handling
- Fallback to defaults

## Error Handling Strategy

### Layered Error Handling
```rust
pub enum TranslatorError {
    HidError(hidapi::HidError),
    DeviceNotFound { vid: u16, pid: u16 },
    FfbError { reason: String },
    // ...
}
```

### Recovery Mechanisms
- Device reconnection on USB errors
- Effect queue recovery on communication failures
- Graceful degradation (disable FFB on errors)

## Performance Characteristics

### Latency Budget
- **Target**: < 2ms end-to-end
- **Input path**: < 1ms (polling + translation)
- **Output path**: < 1ms (parse + IFORCE generation)

### Throughput
- **Input rate**: 1000 reports/second
- **FFB rate**: 1000 updates/second  
- **CPU usage**: < 5% on modern systems

### Memory Usage
- **Baseline**: ~10MB resident
- **Per effect**: ~100 bytes
- **Buffer pools**: Pre-allocated, fixed size

## Platform-Specific Details

### Windows (ViGEm)
```rust
// Create virtual G29 via ViGEm
let vigem_client = vigem_bus_init();
let g29_device = vigem_target_x360_alloc(); // Custom G29 profile
vigem_target_add(vigem_client, g29_device);
```

### Linux (uinput)
```rust
// Create uinput device with G29 descriptor
let uinput_fd = open("/dev/uinput", O_WRONLY | O_NONBLOCK);
ioctl(uinput_fd, UI_SET_EVBIT, EV_ABS);
ioctl(uinput_fd, UI_SET_ABSBIT, ABS_X); // Steering
// ... configure axes and buttons
write(uinput_fd, &uinput_user_dev);
```

### macOS (VirtualHIDDevice)
```rust
// Create IOHIDUserDevice
let properties = IOHIDUserDevice::properties_with_descriptor(&G29_HID_DESCRIPTOR);
let virtual_device = IOHIDUserDevice::create(properties);
```

## Security Considerations

### Device Access
- Exclusive device claiming prevents conflicts
- Permission checks for HID access
- Sandboxing support where available

### Input Validation
- HID report size validation
- Parameter range checking
- Malformed packet handling

### Privilege Management
- Minimal required permissions
- Drop privileges after initialization
- Secure configuration file handling

## Testing Strategy

### Unit Tests
- Protocol translation correctness
- FFB effect parameter validation
- Configuration parsing

### Integration Tests
- End-to-end translation with mock devices
- Platform-specific virtual device creation
- Error condition handling

### Hardware Tests
- Real device compatibility testing
- FFB effect verification
- Performance benchmarking

## Future Enhancements

### Additional Hardware Support
- More Thrustmaster wheel models
- Fanatec wheel support
- Other FFB device families

### Advanced Features
- Telemetry data injection
- Custom effect creation
- Game-specific profiles
- Network-based device sharing

### Performance Optimizations
- SIMD acceleration for translation
- Hardware-accelerated FFB processing
- Zero-copy networking for distributed setups 