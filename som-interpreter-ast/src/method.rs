use crate::class::Class;
use crate::primitives::PrimitiveFn;
use crate::universe::UniverseAST;
use crate::{SOMRef, SOMWeakRef};
use crate::ast::AstMethodDef;
use crate::specialized::down_to_do_node::DownToDoNode;
use crate::specialized::while_node::WhileNode;
use crate::specialized::if_node::IfNode;
use crate::specialized::if_true_if_false_node::IfTrueIfFalseNode;
use crate::specialized::to_by_do_node::ToByDoNode;
use crate::specialized::to_do_node::ToDoNode;
use crate::specialized::trivial_methods::{TrivialGetterMethod, TrivialGlobalMethod, TrivialLiteralMethod, TrivialSetterMethod};

/// The kind of a class method.
#[derive(Clone)]
pub enum MethodKind {
    /// A user-defined method from the AST.
    Defined(AstMethodDef),
    /// An interpreter primitive.
    Primitive(PrimitiveFn),
    /// A trivial literal read
    TrivialLiteral(TrivialLiteralMethod),
    /// A trivial global read
    TrivialGlobal(TrivialGlobalMethod),
    /// A trivial getter method
    TrivialGetter(TrivialGetterMethod),
    /// A trivial setter method
    TrivialSetter(TrivialSetterMethod),
    /// Specialized: whileTrue/whileFalse node.
    While(WhileNode),
    /// Specialized: ifTrue/ifFalse.
    If(IfNode),
    /// Specialized: ifTrue:ifFalse.
    IfTrueIfFalse(IfTrueIfFalseNode),
    /// Specialized: to:do:.
    ToDo(ToDoNode),
    /// Specialized: to:by:do:.
    ToByDo(ToByDoNode),
    /// Specialized: downTo:do:.
    DownToDo(DownToDoNode),
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
    pub holder: SOMWeakRef<Class>,
    pub signature: String,
}

impl Method {
    pub fn class(&self, universe: &UniverseAST) -> SOMRef<Class> {
        if self.is_primitive() {
            universe.primitive_class()
        } else {
            universe.method_class()
        }
    }

    pub fn kind(&self) -> &MethodKind {
        &self.kind
    }

    pub fn holder(&self) -> &SOMWeakRef<Class> {
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
