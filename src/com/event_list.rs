//! IEventList COM implementation.

use parking_lot::Mutex;
use smallvec::SmallVec;
use vst3::{Class, ComWrapper};
use vst3::Steinberg::{
    kInvalidArgument, kResultOk, tresult,
    Vst::{Event, IEventList, IEventListTrait},
};

use crate::types::{
    from_c_event, to_c_event, vst3_event_from_midi, vst3_to_midi_event, vst3_to_note_expression,
    MidiEvent, NoteExpressionValue, Vst3Event,
};

struct Inner {
    events: Vec<Vst3Event>,
    /// Backing storage for `DataEvent.bytes` pointers exposed through
    /// `IEventList::getEvent`. Regrown per frame; cleared when `events` is.
    c_scratch_data: SmallVec<[[u8; 16]; 8]>,
}

impl Default for Inner {
    fn default() -> Self {
        Self {
            events: Vec::with_capacity(256),
            c_scratch_data: SmallVec::new(),
        }
    }
}

pub struct EventList {
    inner: Mutex<Inner>,
}

impl Class for EventList {
    type Interfaces = (IEventList,);
}

impl EventList {
    pub fn new() -> ComWrapper<Self> {
        ComWrapper::new(Self {
            inner: Mutex::new(Inner::default()),
        })
    }

    pub fn update_from_midi(&self, midi_events: &[MidiEvent]) {
        let mut inner = self.inner.lock();
        inner.events.clear();
        inner.c_scratch_data.clear();
        inner
            .events
            .extend(midi_events.iter().filter_map(vst3_event_from_midi));
    }

    pub fn update_from_midi_and_expression(
        &self,
        midi_events: &[MidiEvent],
        note_expressions: &[NoteExpressionValue],
    ) {
        let mut inner = self.inner.lock();
        inner.events.clear();
        inner.c_scratch_data.clear();
        inner
            .events
            .extend(midi_events.iter().filter_map(vst3_event_from_midi));
        for expr in note_expressions {
            inner.events.push(expr.to_vst3_event());
        }
        inner.events.sort_by_key(|e| e.sample_offset());
    }

    pub fn clear(&self) {
        let mut inner = self.inner.lock();
        inner.events.clear();
        inner.c_scratch_data.clear();
    }

    pub fn len(&self) -> usize {
        self.inner.lock().events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.lock().events.is_empty()
    }

    pub fn to_midi_events(&self) -> SmallVec<[MidiEvent; 64]> {
        self.inner
            .lock()
            .events
            .iter()
            .filter_map(vst3_to_midi_event)
            .collect()
    }

    pub fn to_note_expressions(&self) -> SmallVec<[NoteExpressionValue; 16]> {
        self.inner
            .lock()
            .events
            .iter()
            .filter_map(vst3_to_note_expression)
            .collect()
    }
}

impl IEventListTrait for EventList {
    unsafe fn getEventCount(&self) -> i32 {
        self.inner.lock().events.len() as i32
    }

    unsafe fn getEvent(&self, index: i32, e: *mut Event) -> tresult {
        if e.is_null() {
            return kInvalidArgument;
        }
        let mut inner = self.inner.lock();
        if index < 0 || index >= inner.events.len() as i32 {
            return kInvalidArgument;
        }
        let Inner {
            events,
            c_scratch_data,
            ..
        } = &mut *inner;
        let event = events[index as usize];
        *e = to_c_event(&event, c_scratch_data);
        kResultOk
    }

    unsafe fn addEvent(&self, e: *mut Event) -> tresult {
        if e.is_null() {
            return kInvalidArgument;
        }
        let c_event = &*e;
        match from_c_event(c_event) {
            Some(ev) => {
                self.inner.lock().events.push(ev);
                kResultOk
            }
            None => kInvalidArgument,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{EventHeader, NoteOnEvent, K_NOTE_ON_EVENT};

    fn make_note_on() -> NoteOnEvent {
        NoteOnEvent {
            header: EventHeader {
                bus_index: 0,
                sample_offset: 0,
                ppq_position: 0.0,
                flags: 0,
                event_type: K_NOTE_ON_EVENT,
            },
            channel: 0,
            pitch: 60,
            tuning: 0.0,
            velocity: 0.8,
            length: 0,
            note_id: -1,
        }
    }

    #[test]
    fn test_update_from_midi_counts_correctly() {
        let list = EventList::new();
        let midi_events = [MidiEvent::note_on(0, 0, 60, 0x8000).with_frame_offset(0)];
        list.update_from_midi(&midi_events);
        assert_eq!(list.len(), 1);
    }

    #[test]
    fn test_clear_after_update_from_midi() {
        let list = EventList::new();
        let midi_events = [
            MidiEvent::note_on(0, 0, 60, 0x8000).with_frame_offset(0),
            MidiEvent::note_off(0, 0, 60, 0).with_frame_offset(10),
        ];
        list.update_from_midi(&midi_events);
        list.clear();
        assert_eq!(list.len(), 0);
    }

    #[test]
    fn test_get_event_valid() {
        let list = EventList::new();
        list.inner
            .lock()
            .events
            .push(Vst3Event::NoteOn(make_note_on()));
        let ptr = list.to_com_ptr::<IEventList>().unwrap();
        let mut out: Event = unsafe { std::mem::zeroed() };
        let result = unsafe { ptr.getEvent(0, &mut out) };
        assert_eq!(result, kResultOk);
        assert_eq!(out.r#type, K_NOTE_ON_EVENT);
        unsafe {
            assert_eq!(out.__field0.noteOn.pitch, 60);
            assert!((out.__field0.noteOn.velocity - 0.8).abs() < 1e-6);
        }
    }
}
