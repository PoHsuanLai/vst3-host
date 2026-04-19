//! Unit tests for VST3 COM implementations.

use std::ffi::c_void;
use std::time::Duration;

use super::{
    BStream, ComponentHandler, ConnectionPoint, DataExchangeHandler, EventList, HostApplication,
    ParamValueQueueImpl, ParameterChangesImpl, ParameterEditEvent, ProgressEvent, ProgressHandler,
    UnitEvent, UnitHandler,
};
use crate::ffi::{
    DataExchangeBlock, IBStreamVtable, IComponentHandlerVtable, IDataExchangeHandlerVtable,
    IEventListVtable, IParameterChangesVtable, IProgressVtable, IUnitHandlerVtable, K_RESULT_OK,
};
use crate::types::{ParameterChanges, ParameterQueue};

#[test]
fn test_bstream_new() {
    let stream = BStream::new();
    assert_eq!(stream.data(), Vec::<u8>::new());
}

#[test]
fn test_bstream_from_data() {
    let data = vec![1, 2, 3, 4, 5];
    let stream = BStream::from_data(data.clone());
    assert_eq!(stream.data(), data);
}

#[test]
fn test_bstream_into_data() {
    let data = vec![10, 20, 30];
    let stream = BStream::from_data(data.clone());
    assert_eq!(stream.into_data(), data);
}

#[test]
fn test_bstream_default() {
    let stream = BStream::default();
    assert_eq!(stream.data(), Vec::<u8>::new());
}

#[test]
fn test_host_application_new() {
    let _host = HostApplication::new("TestHost");
}

#[test]
fn test_host_application_as_ptr() {
    let host = HostApplication::new("DAWAI");
    let ptr = host.as_ptr();
    assert!(!ptr.is_null());
}

#[test]
fn test_host_application_long_name_truncates() {
    let long_name = "A".repeat(200);
    let _host = HostApplication::new(&long_name);
}

#[test]
fn test_component_handler_new() {
    let (handler, _rx, _prx, _urx) = ComponentHandler::new();
    assert!(!handler.as_ptr().is_null());
}

#[test]
fn test_component_handler_events() {
    let (handler, rx, _prx, _urx) = ComponentHandler::new();
    let ptr = handler.as_ptr();

    unsafe {
        let vtable_ptr = *(ptr as *const *const IComponentHandlerVtable);
        let vtable = &*vtable_ptr;

        let result = (vtable.begin_edit)(ptr, 42);
        assert_eq!(result, K_RESULT_OK);

        let result = (vtable.perform_edit)(ptr, 42, 0.75);
        assert_eq!(result, K_RESULT_OK);

        let result = (vtable.end_edit)(ptr, 42);
        assert_eq!(result, K_RESULT_OK);
    }

    let event1 = rx.recv_timeout(Duration::from_millis(100)).unwrap();
    assert!(matches!(event1, ParameterEditEvent::BeginEdit(42)));

    let event2 = rx.recv_timeout(Duration::from_millis(100)).unwrap();
    assert!(matches!(
        event2,
        ParameterEditEvent::PerformEdit {
            param_id: 42,
            value
        } if (value - 0.75).abs() < 0.001
    ));

    let event3 = rx.recv_timeout(Duration::from_millis(100)).unwrap();
    assert!(matches!(event3, ParameterEditEvent::EndEdit(42)));
}

#[test]
fn test_component_handler_restart() {
    let (handler, rx, _prx, _urx) = ComponentHandler::new();
    let ptr = handler.as_ptr();

    unsafe {
        let vtable_ptr = *(ptr as *const *const IComponentHandlerVtable);
        let vtable = &*vtable_ptr;

        let result = (vtable.restart_component)(ptr, 0b1010);
        assert_eq!(result, K_RESULT_OK);
    }

    let event = rx.recv_timeout(Duration::from_millis(100)).unwrap();
    assert!(matches!(
        event,
        ParameterEditEvent::RestartComponent(0b1010)
    ));
}

#[test]
fn test_progress_handler_new() {
    let (handler, _rx) = ProgressHandler::new();
    assert!(!handler.as_ptr().is_null());
}

#[test]
fn test_progress_handler_events() {
    let (handler, rx) = ProgressHandler::new();
    let ptr = handler.as_ptr();

    unsafe {
        let vtable_ptr = *(ptr as *const *const IProgressVtable);
        let vtable = &*vtable_ptr;

        let mut out_id: u64 = 0;
        let desc: [u16; 5] = [b'T' as u16, b'e' as u16, b's' as u16, b't' as u16, 0];
        let result = (vtable.start)(ptr, 1, desc.as_ptr(), &mut out_id);
        assert_eq!(result, K_RESULT_OK);
        assert_eq!(out_id, 1);

        let result = (vtable.update)(ptr, out_id, 0.5);
        assert_eq!(result, K_RESULT_OK);

        let result = (vtable.finish)(ptr, out_id);
        assert_eq!(result, K_RESULT_OK);
    }

    let event1 = rx.recv_timeout(Duration::from_millis(100)).unwrap();
    match event1 {
        ProgressEvent::Started {
            id,
            progress_type,
            description,
        } => {
            assert_eq!(id, 1);
            assert_eq!(progress_type, 1);
            assert_eq!(description, "Test");
        }
        _ => panic!("Expected Started event"),
    }

    let event2 = rx.recv_timeout(Duration::from_millis(100)).unwrap();
    match event2 {
        ProgressEvent::Updated { id, progress } => {
            assert_eq!(id, 1);
            assert!((progress - 0.5).abs() < 0.001);
        }
        _ => panic!("Expected Updated event"),
    }

    let event3 = rx.recv_timeout(Duration::from_millis(100)).unwrap();
    assert!(matches!(event3, ProgressEvent::Finished { id: 1 }));
}

#[test]
fn test_unit_handler_new() {
    let (handler, _rx) = UnitHandler::new();
    assert!(!handler.as_ptr().is_null());
}

#[test]
fn test_unit_handler_selection() {
    let (handler, rx) = UnitHandler::new();
    let ptr = handler.as_ptr();

    unsafe {
        let vtable_ptr = *(ptr as *const *const IUnitHandlerVtable);
        let vtable = &*vtable_ptr;

        let result = (vtable.notify_unit_selection)(ptr, 5);
        assert_eq!(result, K_RESULT_OK);
    }

    let event = rx.recv_timeout(Duration::from_millis(100)).unwrap();
    assert!(matches!(event, UnitEvent::UnitSelected(5)));
}

#[test]
fn test_unit_handler_program_list() {
    let (handler, rx) = UnitHandler::new();
    let ptr = handler.as_ptr();

    unsafe {
        let vtable_ptr = *(ptr as *const *const IUnitHandlerVtable);
        let vtable = &*vtable_ptr;

        let result = (vtable.notify_program_list_change)(ptr, 10, 3);
        assert_eq!(result, K_RESULT_OK);
    }

    let event = rx.recv_timeout(Duration::from_millis(100)).unwrap();
    match event {
        UnitEvent::ProgramListChanged {
            list_id,
            program_index,
        } => {
            assert_eq!(list_id, 10);
            assert_eq!(program_index, 3);
        }
        _ => panic!("Expected ProgramListChanged event"),
    }
}

#[test]
fn test_data_exchange_handler_new() {
    let (handler, _rx) = DataExchangeHandler::new();
    assert!(!handler.as_ptr().is_null());
}

#[test]
fn test_data_exchange_open_close_queue() {
    let (handler, _rx) = DataExchangeHandler::new();
    let ptr = handler.as_ptr();

    unsafe {
        let vtable_ptr = *(ptr as *const *const IDataExchangeHandlerVtable);
        let vtable = &*vtable_ptr;

        let mut queue_id: u32 = 0;
        let result =
            (vtable.open_queue)(ptr, std::ptr::null_mut(), 1024, 4, 16, 100, &mut queue_id);
        assert_eq!(result, K_RESULT_OK);
        assert_eq!(queue_id, 1);

        let result = (vtable.close_queue)(ptr, 100);
        assert_eq!(result, K_RESULT_OK);
    }
}

#[test]
fn test_data_exchange_lock_free_block() {
    let (handler, rx) = DataExchangeHandler::new();
    let ptr = handler.as_ptr();

    unsafe {
        let vtable_ptr = *(ptr as *const *const IDataExchangeHandlerVtable);
        let vtable = &*vtable_ptr;

        let mut queue_id: u32 = 0;
        let result = (vtable.open_queue)(ptr, std::ptr::null_mut(), 64, 2, 1, 42, &mut queue_id);
        assert_eq!(result, K_RESULT_OK);

        let mut block = DataExchangeBlock {
            data: std::ptr::null_mut(),
            size: 0,
            block_id: 0,
        };
        let result = (vtable.lock_block)(ptr, 42, &mut block);
        assert_eq!(result, K_RESULT_OK);
        assert_eq!(block.size, 64);
        assert!(!block.data.is_null());

        let data_slice = std::slice::from_raw_parts_mut(block.data as *mut u8, 64);
        data_slice[0] = 0xAB;
        data_slice[1] = 0xCD;

        let result = (vtable.free_block)(ptr, 42, block.block_id, 1);
        assert_eq!(result, K_RESULT_OK);
    }

    let data_block = rx.recv_timeout(Duration::from_millis(100)).unwrap();
    assert_eq!(data_block.user_context_id, 42);
    assert_eq!(data_block.data[0], 0xAB);
    assert_eq!(data_block.data[1], 0xCD);
}

#[test]
fn test_event_list_new() {
    let mut event_list = EventList::new();
    assert!(!event_list.as_ptr().is_null());
}

#[test]
fn test_event_list_update_from_midi_counts_correctly() {
    use crate::types::MidiEvent;
    let mut event_list = EventList::new();
    let midi_events = [MidiEvent::note_on(0, 0, 60, 0x8000).with_frame_offset(0)];
    event_list.update_from_midi(&midi_events);

    unsafe {
        let ptr = event_list.as_ptr();
        let vtable_ptr = *(ptr as *const *const IEventListVtable);
        let vtable = &*vtable_ptr;
        assert_eq!((vtable.get_event_count)(ptr), 1);
    }
}

#[test]
fn test_event_list_clear_after_update_from_midi() {
    use crate::types::MidiEvent;
    let mut event_list = EventList::new();
    let midi_events = [
        MidiEvent::note_on(0, 0, 60, 0x8000).with_frame_offset(0),
        MidiEvent::note_off(0, 0, 60, 0).with_frame_offset(10),
    ];
    event_list.update_from_midi(&midi_events);
    event_list.clear();

    unsafe {
        let ptr = event_list.as_ptr();
        let vtable_ptr = *(ptr as *const *const IEventListVtable);
        let vtable = &*vtable_ptr;
        assert_eq!((vtable.get_event_count)(ptr), 0);
    }
}

#[test]
fn test_param_value_queue_from_queue() {
    let mut queue = ParameterQueue::new(42);
    queue.add_point(0, 0.0);
    queue.add_point(128, 0.5);

    let impl_queue = ParamValueQueueImpl::from_queue(&queue);
    assert_eq!(impl_queue.param_id(), 42);
    assert_eq!(impl_queue.len(), 2);
}

#[test]
fn test_param_value_queue_new_empty() {
    let queue = ParamValueQueueImpl::new_empty(1);
    assert_eq!(queue.param_id(), 1);
    assert_eq!(queue.len(), 0);
    assert!(queue.is_empty());
}

#[test]
fn test_param_value_queue_to_queue() {
    let mut queue = ParameterQueue::new(5);
    queue.add_point(0, 0.25);
    queue.add_point(64, 0.75);

    let impl_queue = ParamValueQueueImpl::from_queue(&queue);
    let round_trip = impl_queue.to_queue();

    assert_eq!(round_trip.param_id, 5);
    assert_eq!(round_trip.points.len(), 2);
}

#[test]
fn test_parameter_changes_new_empty() {
    let changes = ParameterChangesImpl::new_empty();
    assert_eq!(changes.len(), 0);
    assert!(changes.is_empty());
}

#[test]
fn test_parameter_changes_from_changes() {
    let mut changes = ParameterChanges::new();
    changes.add_change(1, 0, 0.5);
    changes.add_change(2, 0, 0.75);

    let impl_changes = ParameterChangesImpl::from_changes(&changes);
    assert_eq!(impl_changes.len(), 2);
}

#[test]
fn test_parameter_changes_vtable() {
    let mut impl_changes = ParameterChangesImpl::new_empty();
    let ptr = impl_changes.as_ptr();

    unsafe {
        let vtable_ptr = *(ptr as *const *const IParameterChangesVtable);
        let vtable = &*vtable_ptr;

        let count = (vtable.get_parameter_count)(ptr);
        assert_eq!(count, 0);

        let param_id: u32 = 42;
        let mut index: i32 = -1;
        let queue_ptr = (vtable.add_parameter_data)(ptr, &param_id, &mut index);
        assert!(!queue_ptr.is_null());
        assert_eq!(index, 0);

        let count = (vtable.get_parameter_count)(ptr);
        assert_eq!(count, 1);

        let retrieved_ptr = (vtable.get_parameter_data)(ptr, 0);
        assert!(!retrieved_ptr.is_null());
    }
}

#[test]
fn test_connection_point_new() {
    let cp = ConnectionPoint::new();
    assert!(!cp.as_ptr().is_null());
}

#[test]
fn test_bstream_ref_counting() {
    let stream = BStream::new();
    let ptr = Box::into_raw(stream) as *mut c_void;

    unsafe {
        let vtable_ptr = *(ptr as *const *const IBStreamVtable);
        let vtable = &*vtable_ptr;

        let count = (vtable.add_ref)(ptr);
        assert_eq!(count, 2);

        let count = (vtable.add_ref)(ptr);
        assert_eq!(count, 3);

        let count = (vtable.release)(ptr);
        assert_eq!(count, 2);

        let count = (vtable.release)(ptr);
        assert_eq!(count, 1);

        let count = (vtable.release)(ptr);
        assert_eq!(count, 0);
    }
}

#[test]
fn test_component_handler_ref_counting() {
    let (handler, _rx, _prx, _urx) = ComponentHandler::new();
    let ptr = Box::into_raw(handler) as *mut c_void;

    unsafe {
        let vtable_ptr = *(ptr as *const *const IComponentHandlerVtable);
        let vtable = &*vtable_ptr;

        let count = (vtable.add_ref)(ptr);
        assert_eq!(count, 2);

        let count = (vtable.release)(ptr);
        assert_eq!(count, 1);

        let count = (vtable.release)(ptr);
        assert_eq!(count, 0);
    }
}

#[test]
fn test_component_handler_thread_safe() {
    let (handler, rx, _prx, _urx) = ComponentHandler::new();
    let ptr = handler.as_ptr();

    let handles: Vec<_> = (0..4)
        .map(|i| {
            let ptr = ptr as usize;
            std::thread::spawn(move || {
                let ptr = ptr as *mut c_void;
                unsafe {
                    let vtable_ptr = *(ptr as *const *const IComponentHandlerVtable);
                    let vtable = &*vtable_ptr;

                    for j in 0..10 {
                        (vtable.perform_edit)(ptr, i as u32, j as f64 * 0.1);
                    }
                }
            })
        })
        .collect();

    for h in handles {
        h.join().unwrap();
    }

    let mut count = 0;
    while rx.recv_timeout(Duration::from_millis(100)).is_ok() {
        count += 1;
    }
    assert_eq!(count, 40);

    drop(handler);
}

#[test]
fn test_bstream_vtable_read_write() {
    let mut stream = BStream::new();
    let ptr = stream.as_ptr();

    unsafe {
        let vtable_ptr = *(ptr as *const *const IBStreamVtable);
        let vtable = &*vtable_ptr;

        let data = b"Hello, VST3!";
        let mut written: i32 = 0;
        let result = (vtable.write)(
            ptr,
            data.as_ptr() as *const c_void,
            data.len() as i32,
            &mut written,
        );
        assert_eq!(result, K_RESULT_OK);
        assert_eq!(written, data.len() as i32);

        let mut new_pos: i64 = 0;
        let result = (vtable.seek)(ptr, 0, 0, &mut new_pos);
        assert_eq!(result, K_RESULT_OK);
        assert_eq!(new_pos, 0);

        let mut buffer = [0u8; 32];
        let mut bytes_read: i32 = 0;
        let result = (vtable.read)(
            ptr,
            buffer.as_mut_ptr() as *mut c_void,
            buffer.len() as i32,
            &mut bytes_read,
        );
        assert_eq!(result, K_RESULT_OK);
        assert_eq!(bytes_read, data.len() as i32);
        assert_eq!(&buffer[..data.len()], data);

        let mut pos: i64 = 0;
        let result = (vtable.tell)(ptr, &mut pos);
        assert_eq!(result, K_RESULT_OK);
        assert_eq!(pos, data.len() as i64);
    }
}
