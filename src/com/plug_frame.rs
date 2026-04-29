//! `IPlugFrame` host-side impl. The plugin calls `resizeView` from its
//! UI thread; we forward `onSize` immediately so the view lays out,
//! and queue the size for the main thread to drain.

use crossbeam_channel::{Receiver, Sender};
use vst3::Steinberg::{
    kResultOk, tresult, IPlugFrame, IPlugView, IPlugViewTrait, ViewRect,
};
use vst3::{Class, ComRef, ComWrapper};

use crate::types::EditorSize;

pub(crate) struct HostPlugFrame {
    sender: Sender<EditorSize>,
}

impl Class for HostPlugFrame {
    type Interfaces = (IPlugFrame,);
}

impl HostPlugFrame {
    pub(crate) fn new() -> (ComWrapper<Self>, Receiver<EditorSize>) {
        let (sender, receiver) = crossbeam_channel::unbounded();
        let wrapper = ComWrapper::new(Self { sender });
        (wrapper, receiver)
    }
}

impl vst3::Steinberg::IPlugFrameTrait for HostPlugFrame {
    unsafe fn resizeView(&self, view: *mut IPlugView, new_size: *mut ViewRect) -> tresult {
        if new_size.is_null() {
            return kResultOk;
        }
        let rect = unsafe { *new_size };
        let size = EditorSize {
            width: (rect.right - rect.left) as u32,
            height: (rect.bottom - rect.top) as u32,
        };
        let _ = self.sender.send(size);
        // SAFETY: `view` is a valid IPlugView for the duration of this call.
        if let Some(view_ref) = unsafe { ComRef::<IPlugView>::from_raw(view) } {
            let _ = unsafe { view_ref.onSize(new_size) };
        }
        kResultOk
    }
}
