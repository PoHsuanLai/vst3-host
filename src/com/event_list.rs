//! IEventList COM implementation.

use std::ffi::c_void;
use std::sync::atomic::{AtomicU32, Ordering};

use crate::ffi::{
    DataEvent, IEventListVtable, NoteExpressionValueEvent, NoteOffEvent, NoteOnEvent,
    PolyPressureEvent, Vst3Event, IID_IEVENT_LIST, K_RESULT_OK,
};
use crate::types::{
    vst3_to_midi_event, vst3_to_note_expression, MidiEvent, NoteExpressionValue, Vst3MidiEvent,
};

use smallvec::SmallVec;

/// IEventList COM implementation for providing MIDI events to plugins.
///
/// This struct implements the VST3 IEventList interface, allowing the host
/// to pass MIDI events to plugins during processing.
///
/// # RT-Safety
///
/// The event list is designed for real-time safety:
/// - Pre-allocated vector is reused across process calls
/// - No heap allocations during normal operation
/// - Reference counting uses atomic operations
#[repr(C)]
pub struct EventList {
    #[allow(dead_code)] // Accessed via raw pointer in COM vtable
    vtable: *const IEventListVtable,
    ref_count: AtomicU32,
    events: Vec<Vst3Event>,
}

// Safety: EventList only contains thread-safe types
unsafe impl Send for EventList {}
unsafe impl Sync for EventList {}

impl EventList {
    /// Create a new empty event list.
    pub fn new() -> Box<Self> {
        Box::new(EventList {
            vtable: &EVENT_LIST_VTABLE,
            ref_count: AtomicU32::new(1),
            events: Vec::with_capacity(256),
        })
    }

    /// Update the event list from MIDI events.
    ///
    /// This method clears the existing events and populates the list
    /// with the given MIDI events. It reuses the existing allocation.
    pub fn update_from_midi<E: Vst3MidiEvent>(&mut self, midi_events: &[E]) {
        self.events.clear();
        self.events
            .extend(midi_events.iter().filter_map(|e| e.to_vst3_event()));
    }

    /// Update from MIDI events and note expressions.
    ///
    /// Events are sorted by sample offset after adding.
    pub fn update_from_midi_and_expression<E: Vst3MidiEvent>(
        &mut self,
        midi_events: &[E],
        note_expressions: &[NoteExpressionValue],
    ) {
        self.events.clear();

        // Add MIDI events
        self.events
            .extend(midi_events.iter().filter_map(|e| e.to_vst3_event()));

        // Add note expression events
        for expr in note_expressions {
            self.events.push(expr.to_vst3_event());
        }

        // Sort by sample offset
        self.events.sort_by_key(|e| e.sample_offset());
    }

    /// Clear all events (RT-safe: keeps allocation).
    pub fn clear(&mut self) {
        self.events.clear();
    }

    /// Get the number of events.
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Check if the event list is empty.
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Convert all events to MIDI events.
    ///
    /// Note expression events are skipped.
    pub fn to_midi_events(&self) -> SmallVec<[MidiEvent; 64]> {
        self.events.iter().filter_map(vst3_to_midi_event).collect()
    }

    /// Extract note expression events.
    pub fn to_note_expressions(&self) -> SmallVec<[NoteExpressionValue; 16]> {
        self.events
            .iter()
            .filter_map(vst3_to_note_expression)
            .collect()
    }

    /// Get a raw pointer suitable for passing to VST3 APIs.
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
        -1 // E_NOINTERFACE
    }
}

unsafe extern "system" fn event_list_add_ref(this: *mut c_void) -> u32 {
    let event_list = &*(this as *const EventList);
    event_list.ref_count.fetch_add(1, Ordering::SeqCst) + 1
}

unsafe extern "system" fn event_list_release(this: *mut c_void) -> u32 {
    let event_list = &*(this as *const EventList);
    let count = event_list.ref_count.fetch_sub(1, Ordering::SeqCst) - 1;
    if count == 0 {
        // Drop the box
        let _ = Box::from_raw(this as *mut EventList);
    }
    count
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
    let event_list = &*(this as *const EventList);
    if index < 0 || index >= event_list.events.len() as i32 {
        return -1;
    }

    // Copy the event data to the output pointer
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
    // This implementation allows plugins to add output events
    let event_list = &mut *(this as *mut EventList);

    // Read the event header to determine the type
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
        _ => return -1, // Unknown event type
    }

    K_RESULT_OK
}
