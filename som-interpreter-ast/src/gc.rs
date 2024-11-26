use crate::ast::{AstBlock, AstExpression, AstLiteral, InlinedNode};
use crate::block::Block;
use crate::class::Class;
use crate::frame::{Frame, FrameAccess};
use crate::instance::Instance;
use crate::method::{Method, MethodKind};
use crate::value::Value;
use crate::{FRAME_ARGS_PTR, UNIVERSE_RAW_PTR_CONST};
use log::debug;
use mmtk::util::ObjectReference;
use mmtk::vm::{ObjectModel, SlotVisitor};
use mmtk::Mutator;
use num_bigint::BigInt;
use som_gc::gc_interface::{HasTypeInfoForGC, MMTKtoVMCallbacks, BIGINT_MAGIC_ID, STRING_MAGIC_ID, VECU8_MAGIC_ID};
use som_gc::gcref::Gc;
use som_gc::object_model::VMObjectModel;
use som_gc::slot::SOMSlot;
use som_gc::SOMVM;
use std::ops::Deref;

// Mine. to put in GC headers
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum AstObjMagicId {
    String = STRING_MAGIC_ID as isize,
    BigInt = BIGINT_MAGIC_ID as isize,
    ArrayU8 = VECU8_MAGIC_ID as isize,
    Frame = 100,
    AstBlock = 101,
    ArrayVal = 102,
    Block = 103,
    Method = 104,
    VecAstLiteral = 105,
    Class = 106,
    Instance = 107,
}

// TODO: HACK. this is to be able to define a magic id for it. what we REALLY need is a GCSlice<T> type.
pub struct VecValue(pub Vec<Value>);

impl Deref for VecValue {
    type Target = Vec<Value>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// HACK: ditto.
#[derive(Debug)]
pub struct VecAstLiteral(pub Vec<AstLiteral>);

impl HasTypeInfoForGC for VecValue {
    fn get_magic_gc_id() -> u8 {
        AstObjMagicId::ArrayVal as u8
    }
}

impl HasTypeInfoForGC for AstBlock {
    fn get_magic_gc_id() -> u8 {
        AstObjMagicId::AstBlock as u8
    }
}

impl HasTypeInfoForGC for Block {
    fn get_magic_gc_id() -> u8 {
        AstObjMagicId::Block as u8
    }
}

impl HasTypeInfoForGC for VecAstLiteral {
    fn get_magic_gc_id() -> u8 {
        AstObjMagicId::VecAstLiteral as u8
    }
}

impl HasTypeInfoForGC for Frame {
    fn get_magic_gc_id() -> u8 {
        AstObjMagicId::Frame as u8
    }
}

impl HasTypeInfoForGC for Class {
    fn get_magic_gc_id() -> u8 {
        AstObjMagicId::Class as u8
    }
}

impl HasTypeInfoForGC for Instance {
    fn get_magic_gc_id() -> u8 {
        AstObjMagicId::Instance as u8
    }
}

impl HasTypeInfoForGC for Method {
    fn get_magic_gc_id() -> u8 {
        AstObjMagicId::Method as u8
    }
}

// --- Scanning

fn get_roots_in_mutator_thread(_mutator: &mut Mutator<SOMVM>) -> Vec<SOMSlot> {
    debug!("calling scan_roots_in_mutator_thread");
    unsafe {
        let mut to_process: Vec<SOMSlot> = vec![];

        assert!(
            UNIVERSE_RAW_PTR_CONST.is_some(),
            "GC triggered while the system wasn't finished initializing."
        );

        // walk the frame list.
        let current_frame_addr = &UNIVERSE_RAW_PTR_CONST.unwrap().as_ref().current_frame;
        debug!("scanning root: current_frame");
        to_process.push(SOMSlot::from(current_frame_addr));

        // walk globals (includes core classes, but we also need to move the refs in the CoreClasses class)
        debug!("scanning roots: globals");
        for (_name, val) in UNIVERSE_RAW_PTR_CONST.unwrap().as_ref().globals.iter() {
            if val.is_ptr_type() {
                to_process.push(SOMSlot::from(val.as_mut_ptr()))
            }
        }

        debug!("scanning roots: core classes");
        UNIVERSE_RAW_PTR_CONST.unwrap().as_mut().core.iter().for_each(|cls_ptr| to_process.push(SOMSlot::from(cls_ptr)));

        if let Some(frame_args) = FRAME_ARGS_PTR {
            debug!("scanning roots: frame arguments (frame allocation triggered a GC)");
            for val in frame_args.as_ref() {
                if val.is_ptr_type() {
                    to_process.push(SOMSlot::from(val.as_mut_ptr()))
                }
            }
        }

        debug!("scanning roots: finished");
        to_process
    }
}

pub fn scan_object<'a>(object: ObjectReference, slot_visitor: &'a mut (dyn SlotVisitor<SOMSlot> + 'a)) {
    unsafe {
        let gc_id: &AstObjMagicId = VMObjectModel::ref_to_header(object).as_ref();

        debug!("entering scan_object (type: {:?})", gc_id);

        match gc_id {
            AstObjMagicId::Frame => {
                let frame: &mut Frame = object.to_raw_address().as_mut_ref();

                if !frame.prev_frame.is_empty() {
                    slot_visitor.visit_slot(SOMSlot::from(&frame.prev_frame));
                }

                // ew
                let gcref_frame: Gc<Frame> = Gc::from(object.to_raw_address());

                for i in 0..frame.nbr_locals {
                    let val: &Value = gcref_frame.lookup_local(i);
                    visit_value(val, slot_visitor)
                }

                for i in 0..frame.nbr_args {
                    let val: &Value = gcref_frame.lookup_argument(i);
                    visit_value(val, slot_visitor)
                }
            }
            AstObjMagicId::Class => {
                let class: &mut Class = object.to_raw_address().as_mut_ref();

                slot_visitor.visit_slot(SOMSlot::from(&class.class));

                if class.super_class.is_some() {
                    slot_visitor.visit_slot(SOMSlot::from(class.super_class.as_ref().unwrap()));
                }

                for (_, method_ref) in class.methods.iter() {
                    slot_visitor.visit_slot(SOMSlot::from(method_ref))
                }

                for field_ref in class.fields.iter() {
                    visit_value(field_ref, slot_visitor)
                }
            }
            AstObjMagicId::Method => {
                let method: &mut Method = object.to_raw_address().as_mut_ref();

                slot_visitor.visit_slot(SOMSlot::from(&method.holder));

                match &method.kind {
                    MethodKind::Defined(method_def) => {
                        for expr in &method_def.body.exprs {
                            visit_expr(expr, slot_visitor)
                        }
                    }
                    MethodKind::TrivialLiteral(trivial_lit) => visit_literal(&trivial_lit.literal, slot_visitor),
                    MethodKind::Primitive(_) | MethodKind::TrivialGlobal(_) | MethodKind::TrivialGetter(_) | MethodKind::TrivialSetter(_) => {}
                    MethodKind::Specialized(_) => {} // for now, specialized methods don't contain data that needs to be traced.
                }
            }
            AstObjMagicId::Instance => {
                let instance: &mut Instance = object.to_raw_address().as_mut_ref();

                slot_visitor.visit_slot(SOMSlot::from(&instance.class));

                for val in &instance.fields {
                    visit_value(val, slot_visitor)
                }
            }
            AstObjMagicId::Block => {
                let block: &mut Block = object.to_raw_address().as_mut_ref();
                slot_visitor.visit_slot(SOMSlot::from(&block.frame));
                slot_visitor.visit_slot(SOMSlot::from(&block.block));
            }
            AstObjMagicId::AstBlock => {
                let ast_block: &mut AstBlock = object.to_raw_address().as_mut_ref();

                for expr in &ast_block.body.exprs {
                    visit_expr(expr, slot_visitor)
                }
            }
            AstObjMagicId::VecAstLiteral => {
                let literal_vec: &mut Vec<AstLiteral> = object.to_raw_address().as_mut_ref();
                for lit in literal_vec {
                    visit_literal(lit, slot_visitor)
                }
            }
            AstObjMagicId::ArrayVal => {
                let array_val: &mut Vec<Value> = object.to_raw_address().as_mut_ref();
                for val in array_val {
                    visit_value(val, slot_visitor)
                }
            }
            AstObjMagicId::String | AstObjMagicId::BigInt | AstObjMagicId::ArrayU8 => {} // leaf nodes
        }
    }
}

unsafe fn visit_value<'a>(val: &Value, slot_visitor: &'a mut (dyn SlotVisitor<SOMSlot> + 'a)) {
    if val.is_ptr_type() {
        slot_visitor.visit_slot(SOMSlot::from(val.as_mut_ptr()))
    }
}

fn visit_literal(literal: &AstLiteral, slot_visitor: &mut dyn SlotVisitor<SOMSlot>) {
    match &literal {
        AstLiteral::Symbol(s) | AstLiteral::String(s) => slot_visitor.visit_slot(SOMSlot::from(s)),
        AstLiteral::BigInteger(big_int) => slot_visitor.visit_slot(SOMSlot::from(big_int)),
        AstLiteral::Array(arr) => slot_visitor.visit_slot(SOMSlot::from(arr)),
        AstLiteral::Double(_) | AstLiteral::Integer(_) => {}
    }
}

fn visit_expr(expr: &AstExpression, slot_visitor: &mut dyn SlotVisitor<SOMSlot>) {
    match expr {
        AstExpression::Block(blk) => slot_visitor.visit_slot(SOMSlot::from(blk)),
        AstExpression::Literal(lit) => visit_literal(lit, slot_visitor),
        AstExpression::InlinedCall(inlined_node) => match inlined_node.as_ref() {
            InlinedNode::IfInlined(if_inlined) => {
                visit_expr(&if_inlined.cond_expr, slot_visitor);
                for expr in &if_inlined.body_instrs.exprs {
                    visit_expr(expr, slot_visitor)
                }
            }
            InlinedNode::IfTrueIfFalseInlined(if_true_if_false_inlined) => {
                visit_expr(&if_true_if_false_inlined.cond_expr, slot_visitor);
                for expr in &if_true_if_false_inlined.body_1_instrs.exprs {
                    visit_expr(expr, slot_visitor)
                }
                for expr in &if_true_if_false_inlined.body_2_instrs.exprs {
                    visit_expr(expr, slot_visitor)
                }
            }
            InlinedNode::WhileInlined(while_inlined) => {
                for expr in &while_inlined.cond_instrs.exprs {
                    visit_expr(expr, slot_visitor)
                }
                for expr in &while_inlined.body_instrs.exprs {
                    visit_expr(expr, slot_visitor)
                }
            }
            InlinedNode::OrInlined(or_inlined) => {
                visit_expr(&or_inlined.first, slot_visitor);
                for expr in &or_inlined.second.exprs {
                    visit_expr(expr, slot_visitor)
                }
            }
            InlinedNode::AndInlined(and_inlined) => {
                visit_expr(&and_inlined.first, slot_visitor);
                for expr in &and_inlined.second.exprs {
                    visit_expr(expr, slot_visitor)
                }
            }
            InlinedNode::ToDoInlined(to_do_inlined) => {
                visit_expr(&to_do_inlined.start, slot_visitor);
                visit_expr(&to_do_inlined.end, slot_visitor);
                for expr in &to_do_inlined.body.exprs {
                    visit_expr(expr, slot_visitor);
                }
            }
        },
        AstExpression::LocalExit(expr)
        | AstExpression::NonLocalExit(expr, _)
        | AstExpression::LocalVarWrite(_, expr)
        | AstExpression::ArgWrite(_, _, expr)
        | AstExpression::FieldWrite(_, expr)
        | AstExpression::NonLocalVarWrite(_, _, expr) => visit_expr(expr, slot_visitor),
        AstExpression::UnaryDispatch(dispatch) => {
            visit_expr(&dispatch.dispatch_node.receiver, slot_visitor);
            if let Some(cache) = &dispatch.dispatch_node.inline_cache {
                slot_visitor.visit_slot(SOMSlot::from(&cache.0));
                slot_visitor.visit_slot(SOMSlot::from(&cache.1));
            }
        }
        AstExpression::BinaryDispatch(dispatch) => {
            visit_expr(&dispatch.dispatch_node.receiver, slot_visitor);
            if let Some(cache) = &dispatch.dispatch_node.inline_cache {
                slot_visitor.visit_slot(SOMSlot::from(&cache.0));
                slot_visitor.visit_slot(SOMSlot::from(&cache.1));
            }
            visit_expr(&dispatch.arg, slot_visitor)
        }
        AstExpression::TernaryDispatch(dispatch) => {
            visit_expr(&dispatch.dispatch_node.receiver, slot_visitor);
            if let Some(cache) = &dispatch.dispatch_node.inline_cache {
                slot_visitor.visit_slot(SOMSlot::from(&cache.0));
                slot_visitor.visit_slot(SOMSlot::from(&cache.1));
            }
            visit_expr(&dispatch.arg1, slot_visitor);
            visit_expr(&dispatch.arg2, slot_visitor);
        }
        AstExpression::NAryDispatch(dispatch) => {
            visit_expr(&dispatch.dispatch_node.receiver, slot_visitor);
            if let Some(cache) = &dispatch.dispatch_node.inline_cache {
                slot_visitor.visit_slot(SOMSlot::from(&cache.0));
                slot_visitor.visit_slot(SOMSlot::from(&cache.1));
            }
            for arg in &dispatch.values {
                visit_expr(arg, slot_visitor);
            }
        }
        AstExpression::SuperMessage(super_message) => {
            slot_visitor.visit_slot(SOMSlot::from(&super_message.super_class));
            for arg in &super_message.values {
                visit_expr(arg, slot_visitor);
            }
        }
        AstExpression::GlobalRead(..)
        | AstExpression::LocalVarRead(..)
        | AstExpression::NonLocalVarRead(..)
        | AstExpression::ArgRead(..)
        | AstExpression::FieldRead(..) => {} // leaf nodes
    }
}

fn adapt_post_copy(_object: ObjectReference, _original_obj: ObjectReference) {}

fn get_object_size(object: ObjectReference) -> usize {
    let gc_id: &AstObjMagicId = unsafe { VMObjectModel::ref_to_header(object).as_ref() };

    match gc_id {
        AstObjMagicId::Frame => unsafe {
            let frame: &mut Frame = object.to_raw_address().as_mut_ref();
            Frame::get_true_size(frame.nbr_args, frame.nbr_locals)
        },
        AstObjMagicId::Instance => size_of::<Instance>(),
        AstObjMagicId::String => size_of::<String>(),
        AstObjMagicId::BigInt => size_of::<BigInt>(),
        AstObjMagicId::ArrayU8 => size_of::<Vec<u8>>(),
        AstObjMagicId::AstBlock => size_of::<AstBlock>(),
        AstObjMagicId::VecAstLiteral => size_of::<VecAstLiteral>(),
        AstObjMagicId::ArrayVal => size_of::<Vec<Value>>(),
        AstObjMagicId::Method => size_of::<Method>(),
        AstObjMagicId::Block => size_of::<Block>(),
        AstObjMagicId::Class => size_of::<Class>(),
    }
}

pub fn get_callbacks_for_gc() -> MMTKtoVMCallbacks {
    MMTKtoVMCallbacks {
        scan_object,
        get_roots_in_mutator_thread,
        adapt_post_copy,
        get_object_size,
    }
}
