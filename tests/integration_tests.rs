//! Integration tests for VST3 host with real plugins.
//!
//! These tests require actual VST3 plugins to be installed and are marked
//! with #[ignore] by default. Run with:
//!
//! ```bash
//! cargo test -p vst3-host --test integration_tests -- --ignored
//! ```

use std::path::Path;

use vst3_host::{AudioBuffer, MidiEvent, TransportState, Vst3Instance};

// Common VST3 plugin paths on macOS
const TAL_NOISEMAKER: &str = "/Library/Audio/Plug-Ins/VST3/TAL-NoiseMaker.vst3";
const SURGE_XT: &str = "/Library/Audio/Plug-Ins/VST3/Surge XT.vst3";
const VITAL: &str = "/Library/Audio/Plug-Ins/VST3/Vital.vst3";
const DEXED: &str = "/Library/Audio/Plug-Ins/VST3/Dexed.vst3";

fn find_available_plugin() -> Option<&'static str> {
    [TAL_NOISEMAKER, SURGE_XT, VITAL, DEXED]
        .into_iter()
        .find(|path| Path::new(path).exists())
}

#[test]
#[ignore]
fn test_load_tal_noisemaker() {
    if !Path::new(TAL_NOISEMAKER).exists() {
        eprintln!("TAL-NoiseMaker not installed, skipping");
        return;
    }

    let plugin = Vst3Instance::load(Path::new(TAL_NOISEMAKER), 44100.0, 512);
    match plugin {
        Ok(p) => {
            println!("Loaded: {}", p.info().name);
            println!("Vendor: {}", p.info().vendor);
            println!("Version: {}", p.info().version);
            println!("Audio inputs: {}", p.info().num_inputs);
            println!("Audio outputs: {}", p.info().num_outputs);
            println!("Supports f64: {}", p.supports_f64());
        }
        Err(e) => {
            panic!("Failed to load TAL-NoiseMaker: {:?}", e);
        }
    }
}

#[test]
#[ignore]
fn test_load_any_available_plugin() {
    let path = match find_available_plugin() {
        Some(p) => p,
        None => {
            eprintln!("No VST3 plugins found, skipping");
            return;
        }
    };

    println!("Testing with: {}", path);
    let plugin = Vst3Instance::load(Path::new(path), 44100.0, 512).expect("Failed to load plugin");

    let info = plugin.info();
    assert!(!info.name.is_empty(), "Plugin name should not be empty");
    println!("Successfully loaded: {} by {}", info.name, info.vendor);
}

#[test]
#[ignore]
fn test_process_silence() {
    let path = match find_available_plugin() {
        Some(p) => p,
        None => {
            eprintln!("No VST3 plugins found, skipping");
            return;
        }
    };

    let mut plugin =
        Vst3Instance::load(Path::new(path), 44100.0, 512).expect("Failed to load plugin");

    // Create stereo buffers
    let input_left = vec![0.0f32; 512];
    let input_right = vec![0.0f32; 512];
    let mut output_left = vec![0.0f32; 512];
    let mut output_right = vec![0.0f32; 512];

    let inputs: [&[f32]; 2] = [&input_left, &input_right];
    let mut out_l = output_left.as_mut_slice();
    let mut out_r = output_right.as_mut_slice();
    let mut outputs: [&mut [f32]; 2] = [&mut out_l, &mut out_r];

    let mut buffer = AudioBuffer::new(&inputs, &mut outputs, 512, 44100.0);
    let transport = TransportState::new().tempo(120.0).playing(true);
    let midi: [MidiEvent; 0] = [];

    let _output_events = plugin.process(&mut buffer, &midi, None, &[], &transport);
    println!("Processing completed successfully");
}

#[test]
#[ignore]
fn test_process_with_midi() {
    let path = match find_available_plugin() {
        Some(p) => p,
        None => {
            eprintln!("No VST3 plugins found, skipping");
            return;
        }
    };

    let mut plugin =
        Vst3Instance::load(Path::new(path), 44100.0, 512).expect("Failed to load plugin");

    // Create stereo buffers
    let input_left = vec![0.0f32; 512];
    let input_right = vec![0.0f32; 512];
    let mut output_left = vec![0.0f32; 512];
    let mut output_right = vec![0.0f32; 512];

    let inputs: [&[f32]; 2] = [&input_left, &input_right];
    let mut out_l = output_left.as_mut_slice();
    let mut out_r = output_right.as_mut_slice();
    let mut outputs: [&mut [f32]; 2] = [&mut out_l, &mut out_r];

    let mut buffer = AudioBuffer::new(&inputs, &mut outputs, 512, 44100.0);
    let transport = TransportState::new().tempo(120.0).playing(true);

    let midi = [MidiEvent::note_on(0, 0, 60, 0.8)];

    let _output_events = plugin.process(&mut buffer, &midi, None, &[], &transport);

    // buffer goes out of scope here, releasing borrow on output slices
    let has_output = output_left.iter().any(|&s| s.abs() > 0.0001);
    println!("Plugin produced audio: {}", has_output);
}

#[test]
#[ignore]
fn test_process_multiple_buffers() {
    let path = match find_available_plugin() {
        Some(p) => p,
        None => {
            eprintln!("No VST3 plugins found, skipping");
            return;
        }
    };

    let mut plugin =
        Vst3Instance::load(Path::new(path), 44100.0, 512).expect("Failed to load plugin");

    for i in 0..10 {
        let input_left = vec![0.0f32; 256];
        let input_right = vec![0.0f32; 256];
        let mut output_left = vec![0.0f32; 256];
        let mut output_right = vec![0.0f32; 256];

        let inputs: [&[f32]; 2] = [&input_left, &input_right];
        let mut out_l = output_left.as_mut_slice();
        let mut out_r = output_right.as_mut_slice();
        let mut outputs: [&mut [f32]; 2] = [&mut out_l, &mut out_r];

        let mut buffer = AudioBuffer::new(&inputs, &mut outputs, 256, 44100.0);
        let transport = TransportState::new().tempo(120.0).playing(true);

        let midi: Vec<MidiEvent> = if i == 0 {
            vec![MidiEvent::note_on(0, 0, 60, 0.8)]
        } else if i == 9 {
            vec![MidiEvent::note_off(0, 0, 60, 0.0)]
        } else {
            vec![]
        };

        plugin.process(&mut buffer, &midi, None, &[], &transport);
    }
    println!("Processed 10 buffers successfully");
}

#[test]
#[ignore]
fn test_state_save_load() {
    let path = match find_available_plugin() {
        Some(p) => p,
        None => {
            eprintln!("No VST3 plugins found, skipping");
            return;
        }
    };

    let mut plugin =
        Vst3Instance::load(Path::new(path), 44100.0, 512).expect("Failed to load plugin");

    let state = plugin.get_state();
    match state {
        Ok(data) => {
            println!("Saved state: {} bytes", data.len());
            assert!(!data.is_empty(), "State should not be empty");

            let result = plugin.set_state(&data);
            assert!(result.is_ok(), "Loading state should succeed");
        }
        Err(e) => {
            eprintln!("Save state not supported or failed: {:?}", e);
        }
    }
}

#[test]
#[ignore]
fn test_get_parameters() {
    let path = match find_available_plugin() {
        Some(p) => p,
        None => {
            eprintln!("No VST3 plugins found, skipping");
            return;
        }
    };

    let plugin = Vst3Instance::load(Path::new(path), 44100.0, 512).expect("Failed to load plugin");

    let param_count = plugin.get_parameter_count();
    println!("Plugin has {} parameters", param_count);

    for i in 0..param_count.min(10) {
        let value = plugin.get_parameter(i as u32);
        println!("  [{}] value: {}", i, value);
    }
}

#[test]
#[ignore]
fn test_set_parameter() {
    let path = match find_available_plugin() {
        Some(p) => p,
        None => {
            eprintln!("No VST3 plugins found, skipping");
            return;
        }
    };

    let mut plugin =
        Vst3Instance::load(Path::new(path), 44100.0, 512).expect("Failed to load plugin");

    if plugin.get_parameter_count() > 0 {
        println!("Setting parameter 0 to 0.5");
        plugin.set_parameter(0, 0.5);
        let value = plugin.get_parameter(0);
        println!("Read back value: {}", value);
    }
}

#[test]
#[ignore]
fn test_rapid_process_calls() {
    let path = match find_available_plugin() {
        Some(p) => p,
        None => {
            eprintln!("No VST3 plugins found, skipping");
            return;
        }
    };

    let mut plugin =
        Vst3Instance::load(Path::new(path), 44100.0, 512).expect("Failed to load plugin");

    let transport = TransportState::new().tempo(120.0).playing(true);
    let midi: [MidiEvent; 0] = [];

    let start = std::time::Instant::now();
    for _ in 0..689 {
        let input_left = vec![0.0f32; 64];
        let input_right = vec![0.0f32; 64];
        let mut output_left = vec![0.0f32; 64];
        let mut output_right = vec![0.0f32; 64];

        let inputs: [&[f32]; 2] = [&input_left, &input_right];
        let mut out_l = output_left.as_mut_slice();
        let mut out_r = output_right.as_mut_slice();
        let mut outputs: [&mut [f32]; 2] = [&mut out_l, &mut out_r];

        let mut buffer = AudioBuffer::new(&inputs, &mut outputs, 64, 44100.0);
        plugin.process(&mut buffer, &midi, None, &[], &transport);
    }
    let elapsed = start.elapsed();

    println!("Processed 1 second of audio in {:?}", elapsed);
    assert!(
        elapsed.as_millis() < 1000,
        "Should process faster than real-time"
    );
}
