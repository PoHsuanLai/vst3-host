//! IUnitHandler (v1/v2) standalone COM implementation — ComponentHandler
//! implements these too; this is used only by the unit test harness.

use crossbeam_channel::{Receiver, Sender};
use vst3::{Class, ComWrapper};
use vst3::Steinberg::{
    kResultOk, tresult,
    Vst::{
        IUnitHandler, IUnitHandler2, IUnitHandler2Trait, IUnitHandlerTrait, ProgramListID, UnitID,
    },
};

#[derive(Debug, Clone)]
pub enum UnitEvent {
    UnitSelected(i32),
    ProgramListChanged { list_id: i32, program_index: i32 },
    UnitByBusChanged,
}

pub struct UnitHandler {
    event_sender: Sender<UnitEvent>,
}

impl Class for UnitHandler {
    type Interfaces = (IUnitHandler, IUnitHandler2);
}

impl UnitHandler {
    pub fn new() -> (ComWrapper<Self>, Receiver<UnitEvent>) {
        let (tx, rx) = crossbeam_channel::unbounded();
        let wrapper = ComWrapper::new(Self { event_sender: tx });
        (wrapper, rx)
    }
}

impl IUnitHandlerTrait for UnitHandler {
    unsafe fn notifyUnitSelection(&self, unit_id: UnitID) -> tresult {
        let _ = self.event_sender.send(UnitEvent::UnitSelected(unit_id));
        kResultOk
    }

    unsafe fn notifyProgramListChange(
        &self,
        list_id: ProgramListID,
        program_index: i32,
    ) -> tresult {
        let _ = self.event_sender.send(UnitEvent::ProgramListChanged {
            list_id,
            program_index,
        });
        kResultOk
    }
}

impl IUnitHandler2Trait for UnitHandler {
    unsafe fn notifyUnitByBusChange(&self) -> tresult {
        let _ = self.event_sender.send(UnitEvent::UnitByBusChanged);
        kResultOk
    }
}
