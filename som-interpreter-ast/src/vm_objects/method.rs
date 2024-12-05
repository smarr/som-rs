use crate::ast::AstMethodDef;
use crate::primitives::PrimitiveFn;
use crate::specialized::down_to_do_node::DownToDoNode;
use crate::specialized::to_by_do_node::ToByDoNode;
use crate::specialized::to_do_node::ToDoNode;
use crate::specialized::trivial_methods::{TrivialGetterMethod, TrivialGlobalMethod, TrivialLiteralMethod, TrivialSetterMethod};
use crate::universe::Universe;
use crate::vm_objects::class::Class;
use som_gc::gcref::Gc;
use std::fmt::{Debug, Formatter};

/// The kind of a class method.
// #[derive(Clone, Debug, PartialEq)]
#[derive(Clone)]
pub enum MethodKind {
    /// A user-defined method from the AST.
    Defined(AstMethodDef),
    /// An interpreter primitive.
    Primitive(&'static PrimitiveFn),
    /// A trivial literal read
    TrivialLiteral(TrivialLiteralMethod),
    /// A trivial global read
    TrivialGlobal(TrivialGlobalMethod),
    /// A trivial getter method
    TrivialGetter(TrivialGetterMethod),
    /// A trivial setter method
    TrivialSetter(TrivialSetterMethod),
    /// A call to a specialized method
    Specialized(MethodKindSpecialized),
}

impl Debug for MethodKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("I broke debug for MethodKind since PrimitiveFn can't implement it right now... todo fix")
    }
}

impl PartialEq for MethodKind {
    fn eq(&self, _other: &Self) -> bool {
        todo!("is this comparison used during runtime?")
    }
}

/// MethodKind for specialized methods, which are very common methods whose behaviour we know ahead of time (control flow, for the most part)
/// Importantly, many of them go unused most of the time because we usually inline control flow nodes instead.
#[derive(Debug, Clone, PartialEq)]
pub enum MethodKindSpecialized {
    /// Specialized: to:do:.
    ToDo(ToDoNode),
    /// Specialized: to:by:do:.
    ToByDo(ToByDoNode),
    /// Specialized: downTo:do:.
    DownToDo(DownToDoNode),
}

impl MethodKind {
    /// Whether this invokable is a primitive.
    pub fn is_primitive(&self) -> bool {
        matches!(self, Self::Primitive(_))
    }
}

/// Represents a class method.
#[derive(Debug, Clone)]
pub struct Method {
    pub kind: MethodKind,
    pub holder: Gc<Class>, // it's a weak ref in the original code.
    pub signature: String,
}

impl PartialEq for Method {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind && self.signature == other.signature && (self.holder.ptr == other.holder.ptr)
    }
}

impl Method {
    pub fn class(&self, universe: &Universe) -> Gc<Class> {
        if self.is_primitive() {
            universe.primitive_class()
        } else {
            universe.method_class()
        }
    }

    pub fn kind(&self) -> &MethodKind {
        &self.kind
    }

    pub fn holder(&self) -> &Gc<Class> {
        &self.holder
    }

    pub fn signature(&self) -> &str {
        self.signature.as_str()
    }

    /// Whether this invokable is a primitive.
    pub fn is_primitive(&self) -> bool {
        self.kind.is_primitive()
    }
}
