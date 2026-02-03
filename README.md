# vst3-host

A Rust library for hosting VST3 audio plugins.

## Features

### Core Hosting
- Load VST3 plugins from `.vst3` bundles (macOS, Linux, Windows)
- Process audio in f32 or f64 format
- Send MIDI events to plugins (Note On/Off, CC, Pitch Bend, Aftertouch)
- Note expression support (Volume, Pan, Tuning, Vibrato, Brightness)
- Parameter automation with sample-accurate changes
- Transport/tempo synchronization

### Host Integration
- **IHostApplication** - Provide host name to plugins
- **IComponentHandler** - Receive parameter edit notifications from plugin GUI
- **IComponentHandler2** - Grouped parameter edits for automation
- **IComponentHandler3** - Context menu support
- **IBStream** - VST3-compatible state serialization

### Communication
- **IConnectionPoint** - Processor/controller messaging
- **IMessage/IAttributeList** - Key-value message passing

### Program/Preset Management
- **IUnitHandler** - Unit/bank selection notifications
- **IProgramListData** - Program list persistence

### Other
- **IProgress** - Progress reporting for long operations
- **IMidiMapping** - MIDI CC to parameter mapping
- Cross-platform editor window support

## Usage

```rust
use vst3_host::{Vst3Instance, AudioBuffer, MidiEvent, TransportState};

// Load a VST3 plugin (sample_rate=44100, block_size=512)
let mut plugin = Vst3Instance::load("/path/to/plugin.vst3", 44100.0, 512)?;

// Check capabilities
println!("Name: {}", plugin.info().name);
println!("Supports f64: {}", plugin.supports_f64());

// Prepare audio buffers
let inputs: [&[f32]; 2] = [&[0.0; 512], &[0.0; 512]];
let mut output_left = [0.0f32; 512];
let mut output_right = [0.0f32; 512];
let mut outputs: [&mut [f32]; 2] = [&mut output_left, &mut output_right];

let mut buffer = AudioBuffer::new(&inputs, &mut outputs, 512, 44100.0);

// Process with MIDI events
let midi = vec![MidiEvent::note_on(0, 0, 60, 0.8)];
let transport = TransportState::new().tempo(120.0).playing(true);
let output_midi = plugin.process(&mut buffer, &midi, &transport);
```

## Parameter Edit Events

Receive notifications when the user interacts with the plugin GUI:

```rust
// Poll for parameter edit events from plugin GUI
for event in plugin.poll_param_events() {
    match event {
        ParameterEditEvent::BeginEdit(param_id) => {
            println!("Started editing parameter {}", param_id);
        }
        ParameterEditEvent::PerformEdit { param_id, value } => {
            println!("Parameter {} changed to {}", param_id, value);
        }
        ParameterEditEvent::EndEdit(param_id) => {
            println!("Finished editing parameter {}", param_id);
        }
        _ => {}
    }
}
```

## Custom MIDI Types

If you have your own MIDI event type, implement the `Vst3MidiEvent` trait:

```rust
use vst3_host::{Vst3MidiEvent, ffi::Vst3Event};

impl Vst3MidiEvent for MyMidiEvent {
    fn sample_offset(&self) -> i32 {
        self.offset as i32
    }

    fn to_vst3_event(&self) -> Option<Vst3Event> {
        // Convert your event to a VST3 event
        // ...
    }
}
```

## License

MIT OR Apache-2.0
