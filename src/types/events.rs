//! MIDI event types for VST3 plugin processing.

use crate::ffi::{
    DataEvent, EventHeader, NoteExpressionValueEvent, NoteOffEvent, NoteOnEvent, PolyPressureEvent,
    Vst3Event, K_DATA_EVENT, K_NOTE_EXPRESSION_VALUE_EVENT, K_NOTE_OFF_EVENT, K_NOTE_ON_EVENT,
    K_POLY_PRESSURE_EVENT,
};

pub trait Vst3MidiEvent {
    fn sample_offset(&self) -> i32;
    fn to_vst3_event(&self) -> Option<Vst3Event>;
}

#[derive(Debug, Clone, Copy)]
pub struct Midi1Event {
    pub sample_offset: i32,
    /// 0-15
    pub channel: u8,
    pub data: MidiData,
}

impl Midi1Event {
    pub fn note_on(sample_offset: i32, channel: u8, note: u8, velocity: f32) -> Self {
        Self {
            sample_offset,
            channel,
            data: MidiData::NoteOn { note, velocity },
        }
    }

    pub fn note_off(sample_offset: i32, channel: u8, note: u8, velocity: f32) -> Self {
        Self {
            sample_offset,
            channel,
            data: MidiData::NoteOff { note, velocity },
        }
    }

    pub fn control_change(sample_offset: i32, channel: u8, cc: u8, value: u8) -> Self {
        Self {
            sample_offset,
            channel,
            data: MidiData::ControlChange { cc, value },
        }
    }

    pub fn pitch_bend(sample_offset: i32, channel: u8, value: u16) -> Self {
        Self {
            sample_offset,
            channel,
            data: MidiData::PitchBend { value },
        }
    }

    pub fn program_change(sample_offset: i32, channel: u8, program: u8) -> Self {
        Self {
            sample_offset,
            channel,
            data: MidiData::ProgramChange { program },
        }
    }
}

impl Vst3MidiEvent for Midi1Event {
    fn sample_offset(&self) -> i32 {
        self.sample_offset
    }

    fn to_vst3_event(&self) -> Option<Vst3Event> {
        let channel = self.channel as i16;
        let header = EventHeader {
            bus_index: 0,
            sample_offset: self.sample_offset,
            ppq_position: 0.0,
            flags: 0,
            event_type: 0, // Set below
        };

        match self.data {
            MidiData::NoteOn { note, velocity } => {
                let mut header = header;
                header.event_type = K_NOTE_ON_EVENT;
                Some(Vst3Event::NoteOn(NoteOnEvent {
                    header,
                    channel,
                    pitch: note as i16,
                    tuning: 0.0,
                    velocity,
                    length: 0,
                    note_id: -1,
                }))
            }
            MidiData::NoteOff { note, velocity } => {
                let mut header = header;
                header.event_type = K_NOTE_OFF_EVENT;
                Some(Vst3Event::NoteOff(NoteOffEvent {
                    header,
                    channel,
                    pitch: note as i16,
                    velocity,
                    note_id: -1,
                    tuning: 0.0,
                }))
            }
            MidiData::PolyPressure { note, pressure } => {
                let mut header = header;
                header.event_type = K_POLY_PRESSURE_EVENT;
                Some(Vst3Event::PolyPressure(PolyPressureEvent {
                    header,
                    channel,
                    pitch: note as i16,
                    pressure,
                    note_id: -1,
                }))
            }
            MidiData::ControlChange { cc, value } => {
                let mut header = header;
                header.event_type = K_DATA_EVENT;
                let mut bytes = [0u8; 16];
                bytes[0] = 0xB0 | self.channel;
                bytes[1] = cc;
                bytes[2] = value;
                Some(Vst3Event::Data(DataEvent {
                    header,
                    size: 3,
                    event_type: 0,
                    bytes,
                }))
            }
            MidiData::ProgramChange { program } => {
                let mut header = header;
                header.event_type = K_DATA_EVENT;
                let mut bytes = [0u8; 16];
                bytes[0] = 0xC0 | self.channel;
                bytes[1] = program;
                Some(Vst3Event::Data(DataEvent {
                    header,
                    size: 2,
                    event_type: 0,
                    bytes,
                }))
            }
            MidiData::ChannelPressure { pressure } => {
                let mut header = header;
                header.event_type = K_DATA_EVENT;
                let mut bytes = [0u8; 16];
                bytes[0] = 0xD0 | self.channel;
                bytes[1] = pressure;
                Some(Vst3Event::Data(DataEvent {
                    header,
                    size: 2,
                    event_type: 0,
                    bytes,
                }))
            }
            MidiData::PitchBend { value } => {
                let mut header = header;
                header.event_type = K_DATA_EVENT;
                let mut bytes = [0u8; 16];
                bytes[0] = 0xE0 | self.channel;
                bytes[1] = (value & 0x7F) as u8;
                bytes[2] = ((value >> 7) & 0x7F) as u8;
                Some(Vst3Event::Data(DataEvent {
                    header,
                    size: 3,
                    event_type: 0,
                    bytes,
                }))
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum MidiData {
    NoteOn {
        /// 0-127
        note: u8,
        /// 0.0 to 1.0
        velocity: f32,
    },
    NoteOff {
        /// 0-127
        note: u8,
        /// 0.0 to 1.0
        velocity: f32,
    },
    PolyPressure {
        /// 0-127
        note: u8,
        /// 0.0 to 1.0
        pressure: f32,
    },
    ControlChange {
        /// 0-127
        cc: u8,
        /// 0-127
        value: u8,
    },
    ProgramChange {
        /// 0-127
        program: u8,
    },
    ChannelPressure {
        /// 0-127
        pressure: u8,
    },
    PitchBend {
        /// 14-bit value (0-16383, center = 8192).
        value: u16,
    },
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

pub fn vst3_to_midi_event(event: &Vst3Event) -> Option<Midi1Event> {
    match event {
        Vst3Event::NoteOn(e) => Some(Midi1Event {
            sample_offset: e.header.sample_offset,
            channel: e.channel as u8,
            data: MidiData::NoteOn {
                note: e.pitch as u8,
                velocity: e.velocity,
            },
        }),
        Vst3Event::NoteOff(e) => Some(Midi1Event {
            sample_offset: e.header.sample_offset,
            channel: e.channel as u8,
            data: MidiData::NoteOff {
                note: e.pitch as u8,
                velocity: e.velocity,
            },
        }),
        Vst3Event::PolyPressure(e) => Some(Midi1Event {
            sample_offset: e.header.sample_offset,
            channel: e.channel as u8,
            data: MidiData::PolyPressure {
                note: e.pitch as u8,
                pressure: e.pressure,
            },
        }),
        Vst3Event::Data(e) => {
            if e.size < 2 {
                return None;
            }
            let status = e.bytes[0];
            let channel = status & 0x0F;
            let msg_type = status & 0xF0;

            match msg_type {
                0xB0 if e.size >= 3 => Some(Midi1Event {
                    sample_offset: e.header.sample_offset,
                    channel,
                    data: MidiData::ControlChange {
                        cc: e.bytes[1],
                        value: e.bytes[2],
                    },
                }),
                0xC0 => Some(Midi1Event {
                    sample_offset: e.header.sample_offset,
                    channel,
                    data: MidiData::ProgramChange {
                        program: e.bytes[1],
                    },
                }),
                0xD0 => Some(Midi1Event {
                    sample_offset: e.header.sample_offset,
                    channel,
                    data: MidiData::ChannelPressure {
                        pressure: e.bytes[1],
                    },
                }),
                0xE0 if e.size >= 3 => {
                    let value = ((e.bytes[2] as u16) << 7) | (e.bytes[1] as u16);
                    Some(Midi1Event {
                        sample_offset: e.header.sample_offset,
                        channel,
                        data: MidiData::PitchBend { value },
                    })
                }
                _ => None,
            }
        }
        Vst3Event::NoteExpression(_) => None,
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
