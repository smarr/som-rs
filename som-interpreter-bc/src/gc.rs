use crate::block::{Block, BlockInfo};
use crate::class::Class;
use crate::compiler::Literal;
use crate::frame::Frame;
use crate::instance::{Instance, InstanceAccess};
use crate::method::{Method, MethodKind};
use crate::value::Value;
use crate::{INTERPRETER_RAW_PTR, UNIVERSE_RAW_PTR};
use log::debug;
use mmtk::util::{Address, ObjectReference};
use mmtk::vm::{ObjectModel, SlotVisitor};
use mmtk::Mutator;
use som_gc::gc_interface::{
    HasTypeInfoForGC, MMTKtoVMCallbacks, BIGINT_MAGIC_ID, STRING_MAGIC_ID, VECU8_MAGIC_ID,
};
use som_gc::gcref::GCRef;
use som_gc::object_model::VMObjectModel;
use som_gc::slot::SOMSlot;
use som_gc::SOMVM;

// Mine. to put in GC headers
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum BCObjMagicId {
    String = STRING_MAGIC_ID as isize,
    BigInt = BIGINT_MAGIC_ID as isize,
    ArrayU8 = VECU8_MAGIC_ID as isize,
    Frame = 100,
    BlockInfo = 101,
    Block = 102,
    Class = 103,
    Instance = 104,
    Method = 105,
    ArrayVal = 106,
}

// TODO: HACK. this is to be able to define a magic id for it. what we REALLY need is a GCSlice<T> type.
pub struct VecValue(pub Vec<Value>);

impl HasTypeInfoForGC for VecValue {
    fn get_magic_gc_id() -> u8 {
        BCObjMagicId::ArrayVal as u8
    }
}

impl HasTypeInfoForGC for BlockInfo {
    fn get_magic_gc_id() -> u8 {
        BCObjMagicId::BlockInfo as u8
    }
}

impl HasTypeInfoForGC for Instance {
    fn get_magic_gc_id() -> u8 {
        BCObjMagicId::Instance as u8
    }
}

impl HasTypeInfoForGC for Method {
    fn get_magic_gc_id() -> u8 {
        BCObjMagicId::Method as u8
    }
}

impl HasTypeInfoForGC for Block {
    fn get_magic_gc_id() -> u8 {
        BCObjMagicId::Block as u8
    }
}

impl HasTypeInfoForGC for Class {
    fn get_magic_gc_id() -> u8 {
        BCObjMagicId::Class as u8
    }
}

impl HasTypeInfoForGC for Frame {
    fn get_magic_gc_id() -> u8 {
        BCObjMagicId::Frame as u8
    }
}

// --- Scanning

pub fn visit_value<'a>(val: &Value, slot_visitor: &'a mut (dyn SlotVisitor<SOMSlot> + 'a)) {
    match val.is_ptr_type() {
        true => slot_visitor.visit_slot(SOMSlot::from_value(val.payload())),
        false => {}
    }
}

pub fn scan_object<'a>(
    object: ObjectReference,
    slot_visitor: &'a mut (dyn SlotVisitor<SOMSlot> + 'a),
) {
    unsafe {
        // let _ptr: *mut usize = unsafe { obj_addr.as_mut_ref() };
        let gc_id: &BCObjMagicId = VMObjectModel::ref_to_header(object).as_ref();

        // debug!("entering scan_object (type: {:?})", gc_id);

        match gc_id {
            BCObjMagicId::Frame => {
                let frame: &mut Frame = object.to_raw_address().as_mut_ref();
                debug!("(frame method is: {})", &frame.current_method.signature);

                if !frame.prev_frame.is_empty() {
                    let prev_frame_slot_addr = Address::from_ref(&frame.prev_frame);
                    slot_visitor.visit_slot(SOMSlot::from_address(prev_frame_slot_addr));
                }

                let method_slot_addr = Address::from_ref(&frame.current_method);
                slot_visitor.visit_slot(SOMSlot::from_address(method_slot_addr));

                for i in 0..frame.nbr_locals {
                    let val: &Value = frame.lookup_local(i);
                    visit_value(&val, slot_visitor)
                }

                for i in 0..frame.nbr_args {
                    let val: &Value = frame.lookup_argument(i);
                    visit_value(&val, slot_visitor)
                }

                // this should all really be done in the frame as a custom method. return an iter or something
                let frame_stack_start_addr: Address =
                    object.to_raw_address().add(size_of::<Frame>());
                let mut stack_ptr = frame.stack_ptr;
                while !std::ptr::eq(stack_ptr, frame_stack_start_addr.to_ptr()) {
                    stack_ptr = stack_ptr.sub(1);
                    let stack_val = *stack_ptr;
                    visit_value(&stack_val, slot_visitor)
                }
            }
            BCObjMagicId::Method => {
                let method: &mut Method = object.to_raw_address().as_mut_ref();

                match &method.kind {
                    MethodKind::Defined(method_env) => {
                        for x in &method_env.literals {
                            match x {
                                Literal::Block(blk) => slot_visitor
                                    .visit_slot(SOMSlot::from_address(Address::from_ref(blk))),
                                Literal::String(str) => slot_visitor
                                    .visit_slot(SOMSlot::from_address(Address::from_ref(str))),
                                Literal::BigInteger(bigint) => slot_visitor
                                    .visit_slot(SOMSlot::from_address(Address::from_ref(bigint))),
                                Literal::Array(arr) => slot_visitor
                                    .visit_slot(SOMSlot::from_address(Address::from_ref(arr))),
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                }

                let holder_slot_addr = Address::from_ref(&method.holder);
                slot_visitor.visit_slot(SOMSlot::from_address(holder_slot_addr))
            }
            BCObjMagicId::Class => {
                let class: &mut Class = object.to_raw_address().as_mut_ref();

                slot_visitor.visit_slot(SOMSlot::from_address(Address::from_ref(&class.class)));

                if let Some(_) = class.super_class {
                    slot_visitor.visit_slot(SOMSlot::from_address(Address::from_ref(
                        class.super_class.as_ref().unwrap(),
                    )));
                }

                for (_, method_ref) in class.methods.iter() {
                    slot_visitor.visit_slot(SOMSlot::from_address(Address::from_ref(method_ref)))
                }

                for (_, field_ref) in class.locals.iter() {
                    visit_value(field_ref, slot_visitor)
                }
            }
            BCObjMagicId::Block => {
                let block: &mut Block = object.to_raw_address().as_mut_ref();

                if let Some(frame) = block.frame.as_ref() {
                    slot_visitor.visit_slot(SOMSlot::from_address(Address::from_ref(frame)));
                }

                slot_visitor.visit_slot(SOMSlot::from_address(Address::from_ref(&block.blk_info)));
            }
            BCObjMagicId::Instance => {
                let instance: &mut Instance = object.to_raw_address().as_mut_ref();
                slot_visitor.visit_slot(SOMSlot::from_address(Address::from_ref(&instance.class)));

                // not the cleanest, to be frank
                let gcref_instance: GCRef<Instance> =
                    GCRef::from_u64(object.to_raw_address().as_usize() as u64);
                for i in 0..instance.nbr_fields {
                    let val: Value = gcref_instance.lookup_local(i);
                    visit_value(&val, slot_visitor)
                }
            }
            BCObjMagicId::ArrayVal => {
                let arr: &mut Vec<Value> = object.to_raw_address().as_mut_ref();
                for val in arr {
                    visit_value(val, slot_visitor)
                }
            }
            BCObjMagicId::BlockInfo
            | BCObjMagicId::String
            | BCObjMagicId::ArrayU8
            | BCObjMagicId::BigInt => {
                // leaf nodes: no children.
            }
        }
    }
}

fn get_roots_in_mutator_thread(_mutator: &mut Mutator<SOMVM>) -> Vec<SOMSlot> {
    debug!("calling scan_roots_in_mutator_thread");
    unsafe {
        let mut to_process: Vec<SOMSlot> = vec![];

        // walk the frame list.
        let current_frame_addr = &(*INTERPRETER_RAW_PTR).current_frame;
        debug!(
            "scanning root: current_frame (method: {})",
            current_frame_addr.current_method.signature
        );
        to_process.push(SOMSlot::from_address(Address::from_ref(current_frame_addr)));

        // walk globals (includes core classes)
        debug!("scanning roots: globals");
        for (_name, val) in (*UNIVERSE_RAW_PTR).globals.iter() {
            if val.is_ptr_type() {
                // to_process.push(SOMSlot::from_value(*val));
                to_process.push(SOMSlot::from_value(val.as_u64()));
            }
        }

        debug!("scanning roots: finished");
        to_process
    }
}

pub fn get_callbacks_for_gc() -> MMTKtoVMCallbacks {
    MMTKtoVMCallbacks {
        scan_object_fn: scan_object,
        get_roots_in_mutator_thread_fn: get_roots_in_mutator_thread,
    }
}
