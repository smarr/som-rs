use std::cell::RefCell;
use std::fmt;

use som_core::bytecode::Bytecode;

use crate::class::Class;
use crate::compiler::Literal;
use crate::interpreter::Interpreter;
use crate::primitives::PrimitiveFn;
use crate::universe::Universe;
use crate::value::Value;

use crate::gc::gc_interface::{GCRef, HasTypeInfoForGC};
use crate::gc::object_model::GC_MAGIC_METHOD;
#[cfg(feature = "frame-debug-info")]
use som_core::ast::BlockDebugInfo;

#[derive(Clone)]
pub struct MethodEnv {
    pub literals: Vec<Literal>,
    pub body: Vec<Bytecode>,
    pub inline_cache: RefCell<Vec<Option<(*const Class, GCRef<Method>)>>>,
    pub nbr_locals: usize,
    pub max_stack_size: u8,
    #[cfg(feature = "frame-debug-info")]
    pub block_debug_info: BlockDebugInfo,
}

/// The kind of a class method.
#[derive(Clone)]
pub enum MethodKind {
    /// A user-defined method from the AST.
    Defined(MethodEnv),
    /// An interpreter primitive.
    Primitive(&'static PrimitiveFn),
    /// A non-implemented primitive.
    NotImplemented(String),
}

impl MethodKind {
    /// Whether this invocable is a primitive.
    pub fn is_primitive(&self) -> bool {
        matches!(self, Self::Primitive(_))
    }
}

/// Represents a class method.
#[derive(Clone)]
pub struct Method {
    pub kind: MethodKind,
    pub holder: GCRef<Class>,
    pub signature: String,
}

impl HasTypeInfoForGC for Method {
    fn get_magic_gc_id(&self) -> u8 {
        GC_MAGIC_METHOD
    }
}

impl Method {
    pub fn class(&self, universe: &Universe) -> GCRef<Class> {
        if self.is_primitive() {
            universe.primitive_class()
        } else {
            universe.method_class()
        }
    }

    pub fn kind(&self) -> &MethodKind {
        &self.kind
    }

    pub fn holder(&self) -> &GCRef<Class> {
        &self.holder
    }

    pub fn signature(&self) -> &str {
        self.signature.as_str()
    }

    /// Whether this invocable is a primitive.
    pub fn is_primitive(&self) -> bool {
        self.kind.is_primitive()
    }
}

pub trait Invoke {
    fn invoke(&self, interpreter: &mut Interpreter, universe: &mut Universe, receiver: Value, args: Vec<Value>);
}

impl Invoke for GCRef<Method> {
    fn invoke(
        &self,
        interpreter: &mut Interpreter,
        universe: &mut Universe,
        receiver: Value,
        mut args: Vec<Value>,
    ) {
        match self.to_obj().kind() {
            MethodKind::Defined(_) => {
                let mut frame_args = vec![receiver];
                frame_args.append(&mut args);
                interpreter.push_method_frame_with_args(*self, frame_args.as_slice(), &mut universe.gc_interface);
            }
            MethodKind::Primitive(func) => {
                interpreter.current_frame.to_obj().stack_push(receiver);
                for arg in args {
                    interpreter.current_frame.to_obj().stack_push(arg)
                }
                func(interpreter, universe).expect(&format!("invoking func {} failed", &self.to_obj().signature))
            }
            MethodKind::NotImplemented(_) => todo!(),
        }
    }
}

impl fmt::Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "#{}>>#{} = ",
            self.holder.to_obj().name(),
            self.signature
        )?;
        match &self.kind {
            MethodKind::Defined(env) => {
                writeln!(f, "(")?;
                write!(f, "    <{} locals>", env.nbr_locals)?;
                for bytecode in &env.body {
                    writeln!(f)?;
                    write!(f, "    {}  ", bytecode.padded_name())?;
                    match bytecode {
                        Bytecode::Dup | Bytecode::Dup2 => {}
                        Bytecode::PushLocal(idx) => {
                            write!(f, "local: {}", idx)?;
                        }
                        Bytecode::PushNonLocal(up_idx, idx) => {
                            write!(f, "local: {}, context: {}", idx, up_idx)?;
                        }
                        Bytecode::PushArg(idx) => {
                            write!(f, "argument: {}", idx)?;
                        }
                        Bytecode::PushNonLocalArg(up_idx, idx) => {
                            write!(f, "argument: {}, context: {}", idx, up_idx)?;
                        }
                        Bytecode::PushField(idx) => {
                            write!(f, "index: {}", idx)?;
                        }
                        Bytecode::PushBlock(idx) => {
                            write!(f, "index: {}", idx)?;
                        }
                        Bytecode::PushConstant0
                        | Bytecode::PushConstant1
                        | Bytecode::PushConstant2 => {}
                        Bytecode::PushConstant(idx) => {
                            write!(f, "index: {}, ", idx)?;
                            let constant = &env.literals[*idx as usize];
                            match constant {
                                Literal::Symbol(_) => write!(f, "value: (#Symbol)"),
                                Literal::String(value) => write!(f, "value: (#String) {:?}", value),
                                Literal::Double(value) => write!(f, "value: (#Double) {}", value),
                                Literal::Integer(value) => write!(f, "value: (#Integer) {}", value),
                                Literal::BigInteger(value) => {
                                    write!(f, "value: (#Integer) {}", value.to_obj())
                                }
                                Literal::Array(_) => write!(f, "value: (#Array)"),
                                Literal::Block(_) => write!(f, "value: (#Block)"),
                            }?;
                        }
                        Bytecode::PushGlobal(idx) => {
                            write!(f, "index: {}", idx)?;
                        }
                        Bytecode::Push0 | Bytecode::Push1 | Bytecode::PushNil => {}
                        Bytecode::PushSelf => {}
                        Bytecode::Inc | Bytecode::Dec | Bytecode::Pop => {}
                        Bytecode::PopLocal(up_idx, idx) => {
                            write!(f, "local: {}, context: {}", idx, up_idx)?;
                        }
                        Bytecode::PopArg(up_idx, idx) => {
                            write!(f, "argument: {}, context: {}", idx, up_idx)?;
                        }
                        Bytecode::PopField(idx) => {
                            write!(f, "index: {}", idx)?;
                        }
                        Bytecode::Send1(idx)
                        | Bytecode::Send2(idx)
                        | Bytecode::Send3(idx)
                        | Bytecode::SendN(idx) => {
                            write!(f, "index: {}", idx)?;
                        }
                        Bytecode::SuperSend1(idx)
                        | Bytecode::SuperSend2(idx)
                        | Bytecode::SuperSend3(idx)
                        | Bytecode::SuperSendN(idx) => {
                            write!(f, "index: {}", idx)?;
                        }
                        Bytecode::ReturnLocal => {}
                        Bytecode::ReturnNonLocal(_) => {}
                        Bytecode::ReturnSelf => {}
                        Bytecode::Jump(idx)
                        | Bytecode::JumpBackward(idx) 
                        | Bytecode::JumpOnTruePop(idx)
                        | Bytecode::JumpOnFalsePop(idx)
                        | Bytecode::JumpOnFalseTopNil(idx)
                        | Bytecode::JumpOnTrueTopNil(idx)
                        | Bytecode::JumpIfGreater(idx) => {
                            write!(f, "index: {}", idx)?;
                        }
                        Bytecode::Halt => {}
                    }
                }
                Ok(())
            }
            MethodKind::Primitive(_) => write!(f, "<primitive>"),
            MethodKind::NotImplemented(_) => write!(f, "<primitive>"),
        }
    }
}
