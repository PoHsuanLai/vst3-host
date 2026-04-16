//! IEventList COM implementation.

use std::ffi::c_void;
use std::sync::atomic::AtomicU32;

use super::{com_add_ref, com_release, HasRefCount};
use crate::ffi::{
    DataEvent, IEventListVtable, NoteExpressionValueEvent, NoteOffEvent, NoteOnEvent,
    PolyPressureEvent, Vst3Event, IID_IEVENT_LIST, K_NOT_IMPLEMENTED, K_RESULT_OK,
};
use crate::types::{
    vst3_to_midi_event, vst3_to_note_expression, Midi1Event, NoteExpressionValue, Vst3MidiEvent,
};

use smallvec::SmallVec;

/// Pre-allocated vector is reused across process calls (no heap allocs in steady state).
#[repr(C)]
pub struct EventList {
    #[allow(dead_code)] // Accessed via raw pointer in COM vtable
    vtable: *const IEventListVtable,
    ref_count: AtomicU32,
    events: Vec<Vst3Event>,
}

unsafe impl Send for EventList {}
unsafe impl Sync for EventList {}

impl HasRefCount for EventList {
    fn ref_count(&self) -> &AtomicU32 {
        &self.ref_count
    }
}

impl EventList {
    pub fn new() -> Box<Self> {
        Box::new(EventList {
            vtable: &EVENT_LIST_VTABLE,
            ref_count: AtomicU32::new(1),
            events: Vec::with_capacity(256),
        })
    }

    pub fn update_from_midi<E: Vst3MidiEvent>(&mut self, midi_events: &[E]) {
        self.events.clear();
        self.events
            .extend(midi_events.iter().filter_map(|e| e.to_vst3_event()));
    }

    pub fn update_from_midi_and_expression<E: Vst3MidiEvent>(
        &mut self,
        midi_events: &[E],
        note_expressions: &[NoteExpressionValue],
    ) {
        self.events.clear();
        self.events
            .extend(midi_events.iter().filter_map(|e| e.to_vst3_event()));
        for expr in note_expressions {
            self.events.push(expr.to_vst3_event());
        }
        self.events.sort_by_key(|e| e.sample_offset());
    }

    pub fn clear(&mut self) {
        self.events.clear();
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    pub fn to_midi_events(&self) -> SmallVec<[Midi1Event; 64]> {
        self.events.iter().filter_map(vst3_to_midi_event).collect()
    }

    pub fn to_note_expressions(&self) -> SmallVec<[NoteExpressionValue; 16]> {
        self.events
            .iter()
            .filter_map(vst3_to_note_expression)
            .collect()
    }

    pub fn as_ptr(&mut self) -> *mut c_void {
        self as *mut EventList as *mut c_void
    }
}

impl Default for EventList {
    fn default() -> Self {
        EventList {
            vtable: &EVENT_LIST_VTABLE,
            ref_count: AtomicU32::new(1),
            events: Vec::with_capacity(256),
        }
    }
}

static EVENT_LIST_VTABLE: IEventListVtable = IEventListVtable {
    query_interface: event_list_query_interface,
    add_ref: event_list_add_ref,
    release: event_list_release,
    get_event_count: event_list_get_event_count,
    get_event: event_list_get_event,
    add_event: event_list_add_event,
};

unsafe extern "system" fn event_list_query_interface(
    this: *mut c_void,
    iid: *const [u8; 16],
    obj: *mut *mut c_void,
) -> i32 {
    if (*iid) == IID_IEVENT_LIST || (*iid) == [0; 16] {
        *obj = this;
        event_list_add_ref(this);
        K_RESULT_OK
    } else {
        *obj = std::ptr::null_mut();
        K_NOT_IMPLEMENTED
    }
}

unsafe extern "system" fn event_list_add_ref(this: *mut c_void) -> u32 {
    com_add_ref::<EventList>(this)
}

unsafe extern "system" fn event_list_release(this: *mut c_void) -> u32 {
    com_release::<EventList>(this)
}

unsafe extern "system" fn event_list_get_event_count(this: *mut c_void) -> i32 {
    let event_list = &*(this as *const EventList);
    event_list.events.len() as i32
}

unsafe extern "system" fn event_list_get_event(
    this: *mut c_void,
    index: i32,
    event: *mut c_void,
) -> i32 {
    if event.is_null() {
        return K_NOT_IMPLEMENTED;
    }
    let event_list = &*(this as *const EventList);
    if index < 0 || index >= event_list.events.len() as i32 {
        return K_NOT_IMPLEMENTED;
    }

    match &event_list.events[index as usize] {
        Vst3Event::NoteOn(e) => {
            std::ptr::copy_nonoverlapping(
                e as *const NoteOnEvent as *const u8,
                event as *mut u8,
                std::mem::size_of::<NoteOnEvent>(),
            );
        }
        Vst3Event::NoteOff(e) => {
            std::ptr::copy_nonoverlapping(
                e as *const NoteOffEvent as *const u8,
                event as *mut u8,
                std::mem::size_of::<NoteOffEvent>(),
            );
        }
        Vst3Event::Data(e) => {
            std::ptr::copy_nonoverlapping(
                e as *const DataEvent as *const u8,
                event as *mut u8,
                std::mem::size_of::<DataEvent>(),
            );
        }
        Vst3Event::PolyPressure(e) => {
            std::ptr::copy_nonoverlapping(
                e as *const PolyPressureEvent as *const u8,
                event as *mut u8,
                std::mem::size_of::<PolyPressureEvent>(),
            );
        }
        Vst3Event::NoteExpression(e) => {
            std::ptr::copy_nonoverlapping(
                e as *const NoteExpressionValueEvent as *const u8,
                event as *mut u8,
                std::mem::size_of::<NoteExpressionValueEvent>(),
            );
        }
    }

    K_RESULT_OK
}

unsafe extern "system" fn event_list_add_event(this: *mut c_void, event: *const c_void) -> i32 {
    if event.is_null() {
        return K_NOT_IMPLEMENTED;
    }
    let event_list = &mut *(this as *mut EventList);
    let header = &*(event as *const crate::ffi::EventHeader);

    match header.event_type {
        crate::ffi::K_NOTE_ON_EVENT => {
            let note_on = &*(event as *const NoteOnEvent);
            event_list.events.push(Vst3Event::NoteOn(*note_on));
        }
        crate::ffi::K_NOTE_OFF_EVENT => {
            let note_off = &*(event as *const NoteOffEvent);
            event_list.events.push(Vst3Event::NoteOff(*note_off));
        }
        crate::ffi::K_DATA_EVENT => {
            let data = &*(event as *const DataEvent);
            event_list.events.push(Vst3Event::Data(*data));
        }
        crate::ffi::K_POLY_PRESSURE_EVENT => {
            let poly = &*(event as *const PolyPressureEvent);
            event_list.events.push(Vst3Event::PolyPressure(*poly));
        }
        crate::ffi::K_NOTE_EXPRESSION_VALUE_EVENT => {
            let expr = &*(event as *const NoteExpressionValueEvent);
            event_list.events.push(Vst3Event::NoteExpression(*expr));
        }
        _ => return K_NOT_IMPLEMENTED,
    }

    K_RESULT_OK
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_note_on() -> NoteOnEvent {
        NoteOnEvent {
            header: crate::ffi::EventHeader {
                bus_index: 0,
                sample_offset: 0,
                ppq_position: 0.0,
                flags: 0,
                event_type: crate::ffi::K_NOTE_ON_EVENT,
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
    fn test_get_event_null_pointer() {
        let mut list = EventList::new();
        list.events.push(Vst3Event::NoteOn(make_note_on()));
        let ptr = list.as_ptr();
        unsafe {
            let result = event_list_get_event(ptr, 0, std::ptr::null_mut());
            assert_ne!(result, K_RESULT_OK);
        }
    }

    #[test]
    fn test_get_event_negative_index() {
        let mut list = EventList::new();
        list.events.push(Vst3Event::NoteOn(make_note_on()));
        let ptr = list.as_ptr();
        let mut buf = [0u8; 64];
        unsafe {
            let result = event_list_get_event(ptr, -1, buf.as_mut_ptr() as *mut c_void);
            assert_ne!(result, K_RESULT_OK);
        }
    }

    #[test]
    fn test_get_event_out_of_bounds() {
        let mut list = EventList::new();
        list.events.push(Vst3Event::NoteOn(make_note_on()));
        let ptr = list.as_ptr();
        let mut buf = [0u8; 64];
        unsafe {
            let result = event_list_get_event(ptr, 5, buf.as_mut_ptr() as *mut c_void);
            assert_ne!(result, K_RESULT_OK);
        }
    }

    #[test]
    fn test_get_event_valid() {
        let mut list = EventList::new();
        list.events.push(Vst3Event::NoteOn(make_note_on()));
        let ptr = list.as_ptr();
        let mut out = std::mem::MaybeUninit::<NoteOnEvent>::uninit();
        unsafe {
            let result = event_list_get_event(ptr, 0, out.as_mut_ptr() as *mut c_void);
            assert_eq!(result, K_RESULT_OK);
            let out = out.assume_init();
            assert_eq!(out.pitch, 60);
            assert!((out.velocity - 0.8).abs() < 1e-6);
        }
    }

    #[test]
    fn test_add_event_null_pointer() {
        let mut list = EventList::new();
        let ptr = list.as_ptr();
        unsafe {
            let result = event_list_add_event(ptr, std::ptr::null());
            assert_ne!(result, K_RESULT_OK);
        }
        assert!(list.events.is_empty());
    }

    #[test]
    fn test_add_event_valid() {
        let mut list = EventList::new();
        let ptr = list.as_ptr();
        let note = make_note_on();
        unsafe {
            let result = event_list_add_event(ptr, &note as *const NoteOnEvent as *const c_void);
            assert_eq!(result, K_RESULT_OK);
        }
        assert_eq!(list.events.len(), 1);
    }

    #[test]
    fn test_event_count() {
        let mut list = EventList::new();
        let ptr = list.as_ptr();
        unsafe {
            assert_eq!(event_list_get_event_count(ptr), 0);
        }
        list.events.push(Vst3Event::NoteOn(make_note_on()));
        list.events.push(Vst3Event::NoteOn(make_note_on()));
        unsafe {
            assert_eq!(event_list_get_event_count(ptr), 2);
        }
    }
}
