//! MIDI event types for VST3 plugin processing.
//!
//! VST3's native event list is richer than raw MIDI-1 wire (typed
//! note-on/off/poly-pressure structs with f32 velocity plus generic `Data`
//! events for CC/ProgramChange/ChannelPressure/PitchBend). These helpers
//! bridge it to the workspace's canonical [`tutti_midi::MidiEvent`] UMP
//! type: one `MidiEvent` → one `Vst3Event`, round-trippable.

use crate::ffi::{
    DataEvent, EventHeader, NoteExpressionValueEvent, NoteOffEvent, NoteOnEvent, PolyPressureEvent,
    Vst3Event, K_DATA_EVENT, K_NOTE_EXPRESSION_VALUE_EVENT, K_NOTE_OFF_EVENT, K_NOTE_ON_EVENT,
    K_POLY_PRESSURE_EVENT,
};

pub use tutti_midi::MidiEvent;

/// Build a [`Vst3Event`] from a Tutti UMP [`MidiEvent`].
///
/// NoteOn/NoteOff/PolyPressure go to VST3's typed event structs (velocity
/// and pressure are MIDI-1 u7 values divided by 127 into the VST3 f32 unit
/// range). Everything else (CC, ProgramChange, ChannelPressure, PitchBend)
/// becomes a generic `Data` event carrying the 3-byte MIDI-1 wire form.
///
/// Returns `None` for UMP variants with no MIDI-1 wire representation
/// (per-note controllers, SysEx, utility).
pub fn vst3_event_from_midi(event: &MidiEvent) -> Option<Vst3Event> {
    let (bytes, _len) = event.to_midi1_bytes()?;
    let sample_offset = event.frame_offset as i32;
    let status = bytes[0];
    let channel = (status & 0x0F) as i16;
    let header = EventHeader {
        bus_index: 0,
        sample_offset,
        ppq_position: 0.0,
        flags: 0,
        event_type: 0,
    };

    match status & 0xF0 {
        0x90 => {
            let mut h = header;
            h.event_type = K_NOTE_ON_EVENT;
            Some(Vst3Event::NoteOn(NoteOnEvent {
                header: h,
                channel,
                pitch: bytes[1] as i16,
                tuning: 0.0,
                velocity: bytes[2] as f32 / 127.0,
                length: 0,
                note_id: -1,
            }))
        }
        0x80 => {
            let mut h = header;
            h.event_type = K_NOTE_OFF_EVENT;
            Some(Vst3Event::NoteOff(NoteOffEvent {
                header: h,
                channel,
                pitch: bytes[1] as i16,
                velocity: bytes[2] as f32 / 127.0,
                note_id: -1,
                tuning: 0.0,
            }))
        }
        0xA0 => {
            let mut h = header;
            h.event_type = K_POLY_PRESSURE_EVENT;
            Some(Vst3Event::PolyPressure(PolyPressureEvent {
                header: h,
                channel,
                pitch: bytes[1] as i16,
                pressure: bytes[2] as f32 / 127.0,
                note_id: -1,
            }))
        }
        _ => {
            let mut h = header;
            h.event_type = K_DATA_EVENT;
            let mut data = [0u8; 16];
            data[..3].copy_from_slice(&bytes);
            Some(Vst3Event::Data(DataEvent {
                header: h,
                size: 3,
                event_type: 0,
                bytes: data,
            }))
        }
    }
}

/// Convert a [`Vst3Event`] back to a Tutti UMP [`MidiEvent`].
///
/// Typed NoteOn/NoteOff/PolyPressure events are re-encoded to MIDI-1 wire
/// bytes (with VST3's f32 unit range scaled back to u7); `Data` events
/// pass their bytes through. [`MidiEvent::from_midi1_bytes`] then does the
/// MIDI 1.0 → 2.0 resolution upconversion.
///
/// Returns `None` for VST3 event types with no MIDI-1 equivalent
/// (`NoteExpression`).
pub fn vst3_to_midi_event(event: &Vst3Event) -> Option<MidiEvent> {
    match event {
        Vst3Event::NoteOn(e) => {
            let velocity_u7 = (e.velocity * 127.0).clamp(0.0, 127.0) as u8;
            let status = 0x90 | ((e.channel as u8) & 0x0F);
            let frame = e.header.sample_offset.max(0) as u32;
            MidiEvent::from_midi1_bytes(frame, &[status, e.pitch as u8 & 0x7F, velocity_u7])
        }
        Vst3Event::NoteOff(e) => {
            let velocity_u7 = (e.velocity * 127.0).clamp(0.0, 127.0) as u8;
            let status = 0x80 | ((e.channel as u8) & 0x0F);
            let frame = e.header.sample_offset.max(0) as u32;
            MidiEvent::from_midi1_bytes(frame, &[status, e.pitch as u8 & 0x7F, velocity_u7])
        }
        Vst3Event::PolyPressure(e) => {
            let pressure_u7 = (e.pressure * 127.0).clamp(0.0, 127.0) as u8;
            let status = 0xA0 | ((e.channel as u8) & 0x0F);
            let frame = e.header.sample_offset.max(0) as u32;
            MidiEvent::from_midi1_bytes(frame, &[status, e.pitch as u8 & 0x7F, pressure_u7])
        }
        Vst3Event::Data(e) => {
            if e.size < 2 {
                return None;
            }
            let frame = e.header.sample_offset.max(0) as u32;
            MidiEvent::from_midi1_bytes(frame, &e.bytes[..e.size as usize])
        }
        Vst3Event::NoteExpression(_) => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoteExpressionType {
    /// Volume expression (0.0 = -inf dB, 0.5 = 0dB, 1.0 = +6dB).
    Volume,
    /// Pan expression (0.0 = left, 0.5 = center, 1.0 = right).
    Pan,
    /// Tuning in semitones (-120 to +120 mapped to 0.0-1.0).
    Tuning,
    /// Vibrato intensity (0.0 = none, 1.0 = max).
    Vibrato,
    /// Brightness/filter cutoff (0.0 = dark, 1.0 = bright).
    Brightness,
}

impl NoteExpressionType {
    pub fn to_type_id(self) -> u32 {
        match self {
            NoteExpressionType::Volume => 0,
            NoteExpressionType::Pan => 1,
            NoteExpressionType::Tuning => 2,
            NoteExpressionType::Vibrato => 3,
            NoteExpressionType::Brightness => 4,
        }
    }

    pub fn from_type_id(id: u32) -> Option<Self> {
        match id {
            0 => Some(NoteExpressionType::Volume),
            1 => Some(NoteExpressionType::Pan),
            2 => Some(NoteExpressionType::Tuning),
            3 => Some(NoteExpressionType::Vibrato),
            4 => Some(NoteExpressionType::Brightness),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct NoteExpressionValue {
    pub sample_offset: i32,
    pub note_id: i32,
    pub expression_type: NoteExpressionType,
    /// 0.0 to 1.0
    pub value: f64,
}

impl NoteExpressionValue {
    pub fn to_vst3_event(&self) -> Vst3Event {
        let header = EventHeader {
            bus_index: 0,
            sample_offset: self.sample_offset,
            ppq_position: 0.0,
            flags: 0,
            event_type: K_NOTE_EXPRESSION_VALUE_EVENT,
        };

        Vst3Event::NoteExpression(NoteExpressionValueEvent {
            header,
            note_id: self.note_id,
            type_id: self.expression_type.to_type_id(),
            value: self.value,
        })
    }
}

pub fn vst3_to_note_expression(event: &Vst3Event) -> Option<NoteExpressionValue> {
    match event {
        Vst3Event::NoteExpression(e) => {
            let expression_type = NoteExpressionType::from_type_id(e.type_id)?;
            Some(NoteExpressionValue {
                sample_offset: e.header.sample_offset,
                note_id: e.note_id,
                expression_type,
                value: e.value,
            })
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    //! MIDI round-trip tests through `vst3_event_from_midi` + `vst3_to_midi_event`.
    //!
    //! Verifies each MIDI-1 message shape lands in the right VST3 event
    //! variant (typed NoteOn/Off/PolyPressure vs. generic `Data`) and
    //! round-trips back to an equivalent `MidiEvent`.

    use super::*;

    #[test]
    fn note_on_lands_in_note_on_variant() {
        let event = MidiEvent::note_on(0, 3, 60, 0x8000).with_frame_offset(5);
        let vst3 = vst3_event_from_midi(&event).expect("NoteOn should convert");
        match &vst3 {
            Vst3Event::NoteOn(e) => {
                assert_eq!(e.channel, 3);
                assert_eq!(e.pitch, 60);
                assert_eq!(e.header.sample_offset, 5);
                assert!(e.velocity > 0.0, "expected non-zero velocity");
            }
            _ => panic!("expected NoteOn variant"),
        }

        let back = vst3_to_midi_event(&vst3).expect("round-trip");
        assert!(back.is_note_on());
        assert_eq!(back.note(), Some(60));
        assert_eq!(back.frame_offset, 5);
    }

    #[test]
    fn note_off_lands_in_note_off_variant() {
        let event = MidiEvent::note_off(0, 0, 72, 0x4000).with_frame_offset(10);
        let vst3 = vst3_event_from_midi(&event).expect("NoteOff should convert");
        match &vst3 {
            Vst3Event::NoteOff(e) => {
                assert_eq!(e.pitch, 72);
                assert_eq!(e.header.sample_offset, 10);
            }
            _ => panic!("expected NoteOff variant"),
        }

        let back = vst3_to_midi_event(&vst3).expect("round-trip");
        assert!(back.is_note_off());
        assert_eq!(back.note(), Some(72));
    }

    #[test]
    fn poly_pressure_lands_in_poly_pressure_variant() {
        use tutti_midi::convert::midi1_cc_to_midi2;
        let event =
            MidiEvent::poly_pressure(0, 1, 60, midi1_cc_to_midi2(100)).with_frame_offset(0);
        let vst3 = vst3_event_from_midi(&event).expect("PolyPressure should convert");
        assert!(matches!(vst3, Vst3Event::PolyPressure(_)));
        let back = vst3_to_midi_event(&vst3).expect("round-trip");
        assert_eq!(back.note(), Some(60));
    }

    #[test]
    fn cc_falls_through_to_data_event() {
        use tutti_midi::convert::midi1_cc_to_midi2;
        let event = MidiEvent::cc(0, 2, 74, midi1_cc_to_midi2(100));
        let vst3 = vst3_event_from_midi(&event).expect("CC should convert");
        match &vst3 {
            Vst3Event::Data(e) => {
                assert_eq!(e.size, 3);
                assert_eq!(e.bytes[0], 0xB0 | 2);
                assert_eq!(e.bytes[1], 74);
                assert_eq!(e.bytes[2], 100);
            }
            _ => panic!("expected Data variant for CC"),
        }
    }

    #[test]
    fn pitch_bend_falls_through_to_data_event() {
        use tutti_midi::convert::midi1_pitch_bend_to_midi2;
        let event = MidiEvent::pitch_bend(0, 0, midi1_pitch_bend_to_midi2(8192));
        let vst3 = vst3_event_from_midi(&event).expect("PitchBend should convert");
        match &vst3 {
            Vst3Event::Data(e) => {
                assert_eq!(e.size, 3);
                assert_eq!(e.bytes[0] & 0xF0, 0xE0);
                let value = (e.bytes[1] as u16) | ((e.bytes[2] as u16) << 7);
                assert_eq!(value, 8192, "PitchBend should round-trip to center");
            }
            _ => panic!("expected Data variant for PitchBend"),
        }
    }

    #[test]
    fn program_change_falls_through_to_data_event() {
        let event = MidiEvent::program_change(0, 9, 42, None);
        let vst3 = vst3_event_from_midi(&event).expect("ProgramChange should convert");
        match &vst3 {
            Vst3Event::Data(e) => {
                assert_eq!(e.size, 3);
                assert_eq!(e.bytes[0], 0xC0 | 9);
                assert_eq!(e.bytes[1], 42);
            }
            _ => panic!("expected Data variant for ProgramChange"),
        }
    }

    #[test]
    fn note_expression_is_not_a_midi_event() {
        let expr = NoteExpressionValue {
            sample_offset: 0,
            note_id: 1,
            expression_type: NoteExpressionType::Tuning,
            value: 0.5,
        };
        let vst3 = expr.to_vst3_event();
        assert!(vst3_to_midi_event(&vst3).is_none());
    }

    #[test]
    fn data_event_with_truncated_size_fails_gracefully() {
        let e = DataEvent {
            header: EventHeader {
                bus_index: 0,
                sample_offset: 0,
                ppq_position: 0.0,
                flags: 0,
                event_type: K_DATA_EVENT,
            },
            size: 1,
            event_type: 0,
            bytes: [0xB0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        };
        assert!(vst3_to_midi_event(&Vst3Event::Data(e)).is_none());
    }
}
