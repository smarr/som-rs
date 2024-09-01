use std::cell::RefCell;
use std::fmt::{Debug, Display, Formatter};
use std::rc::Rc;
use indenter::indented;
use std::fmt::Write;
use crate::class::Class;
use crate::method::Method;
use crate::SOMRef;
use crate::specialized::inlined::and_inlined_node::AndInlinedNode;
use crate::specialized::inlined::if_inlined_node::IfInlinedNode;
use crate::specialized::inlined::if_true_if_false_inlined_node::IfTrueIfFalseInlinedNode;
use crate::specialized::inlined::or_inlined_node::OrInlinedNode;
use crate::specialized::inlined::while_inlined_node::WhileInlinedNode;

#[derive(Debug, Clone, PartialEq)]
pub enum InlinedNode {
    IfInlined(IfInlinedNode),
    IfTrueIfFalseInlined(IfTrueIfFalseInlinedNode),
    WhileInlined(WhileInlinedNode),
    OrInlined(OrInlinedNode),
    AndInlined(AndInlinedNode)
}

#[derive(Debug, Clone, PartialEq)]
pub struct AstBody {
    pub exprs: Vec<AstExpression>,
}

// identical but using refs as
#[derive(Debug, Clone, PartialEq)]
pub enum AstExpression {
    GlobalRead(String),
    LocalVarRead(usize),
    NonLocalVarRead(usize, usize),
    ArgRead(usize, usize),
    FieldRead(usize),
    LocalVarWrite(usize, Box<AstExpression>),
    NonLocalVarWrite(usize, usize, Box<AstExpression>),
    ArgWrite(usize, usize, Box<AstExpression>),
    FieldWrite(usize, Box<AstExpression>),
    Message(Box<AstMessageDispatch>),
    SuperMessage(Box<AstSuperMessage>),
    BinaryOp(Box<AstBinaryOpDispatch>),
    LocalExit(Box<AstExpression>),
    NonLocalExit(Box<AstExpression>, usize),
    Literal(som_core::ast::Literal),
    Block(Rc<RefCell<AstBlock>>), // Rc here, while it's not an Rc in the parser/som-core AST since BC doesn't need that same Rc.
    /// Call to an inlined method node (no dispatching like a message would)
    InlinedCall(Box<InlinedNode>),
    // todo we might want a SEQUENCENODE of some kind. instead of relying on AstBody at all, actually.
}

#[derive(Debug, Clone, PartialEq)]
pub struct AstTerm {
    pub body: AstBody,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AstBlock {
    pub nbr_params: usize,
    pub nbr_locals: usize,
    pub body: AstBody
}

#[derive(Clone)]
pub struct AstBinaryOpDispatch {
    /// Represents the operator symbol.
    pub op: String,
    /// Represents the left-hand side.
    pub lhs: AstExpression,
    /// Represents the right-hand side.
    pub rhs: AstExpression,
    /// Inline cache
    pub inline_cache: Option<CacheEntry>,
}

impl Debug for AstBinaryOpDispatch {
    fn fmt(&self, _f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!("no debug for binary op, todo")
    }
}

impl PartialEq for AstBinaryOpDispatch {
    fn eq(&self, other: &Self) -> bool {
        self.lhs == other.lhs && self.rhs == other.rhs && self.op == other.op
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AstMessage {
    pub receiver: AstExpression,
    pub signature: String,
    pub values: Vec<AstExpression>,
}

type CacheEntry = (*const Class, SOMRef<Method>);
#[derive(Clone)]
pub struct AstMessageDispatch {
    pub message: AstMessage,
    pub inline_cache: Option<CacheEntry>,
    // pub inline_cache: Box<[Option<CacheEntry>; INLINE_CACHE_SIZE]>,
}

impl Debug for AstMessageDispatch {
    fn fmt(&self, _f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!("no debug for astmessagecall, todo")
    }
}

impl PartialEq for AstMessageDispatch {
    fn eq(&self, other: &Self) -> bool {
        self.message == other.message
    }
}


impl AstMessageDispatch {
    pub fn from_message(message: AstMessage) -> Self {
        Self {
            message,
            inline_cache: None
        }
    }

    // todo remove?
    // pub fn lookup_cache(&self, key: &ClassPtr) -> Option<SOMRef<Method>> {
    //     for (cached_class, cached_method) in self.inline_cache.iter().flatten() {
    //         if cached_class == key {
    //             return Some(Rc::clone(&cached_method));
    //         }
    //     }
    // 
    //     None
    // }
    // 
    // pub fn cache_some_entry(&mut self, class_ptr: &ClassPtr, method_ptr: SOMRef<Method>) {
    //     for cache_elem in self.inline_cache.iter_mut() {
    //         if cache_elem.is_none() {
    //             *cache_elem = Some((*class_ptr, method_ptr));
    //             return;
    //         }
    //     }
    // }

    /* #[cfg(debug_assertions)]
     pub fn debug_cache_len(&self) -> usize {
         let mut i = 0;
         let mut cache_elem = self.inline_cache.as_ref();

         while let Some(elem) = cache_elem {
             // dbg!(&cache_elem.unwrap().class_ptr);
             cache_elem = elem.next.as_ref();
             i += 1;
         }

         // dbg!();

         i
     }*/
}

#[derive(Debug, Clone, PartialEq)]
pub struct AstSuperMessage {
    pub receiver_name: String,
    pub is_static_class_call: bool,
    pub signature: String,
    pub values: Vec<AstExpression>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AstMethodDef {
    /// The method's signature (eg. `println`, `at:put:` or `==`).
    pub signature: String,
    /// The method's body.
    pub body: AstBody,
    /// Number of local variables
    pub locals_nbr: usize,
}

// ----------------

impl Display for AstMethodDef {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("Method {} ({} locals):", &self.signature, self.locals_nbr))?;
        f.write_str(self.body.to_string().as_str())
    }
}

impl Display for AstBody {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "AstBody:")?;
        for expr in &self.exprs {
            write!(indented(f), "{}", expr)?;
        }
        Ok(())
    }
}

impl Display for AstBlock {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "AstBlock({} params, {} locals):", self.nbr_params, self.nbr_locals)?;
        for expr in &self.body.exprs {
            write!(indented(f), "{}", expr)?;
        }
        Ok(())
    }
}

// probably not using the indenter lib as one should? though it works. I've given it as little effort as possible.
impl Display for AstExpression {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            AstExpression::GlobalRead(name) => writeln!(f, "GlobalRead({})", name),
            AstExpression::LocalVarRead(index) => writeln!(f, "LocalVarRead({})", index),
            AstExpression::NonLocalVarRead(level, index) => writeln!(f, "NonLocalVarRead({}, {})", level, index),
            AstExpression::ArgRead(level, index) => writeln!(f, "ArgRead({}, {})", level, index),
            AstExpression::FieldRead(index) => writeln!(f, "FieldRead({})", index),
            AstExpression::LocalVarWrite(index, expr) => {
                writeln!(f, "LocalVarWrite({}):", index)?;
                write!(indented(f), "{}", expr)
            }
            AstExpression::NonLocalVarWrite(level, index, expr) => {
                writeln!(f, "NonLocalVarWrite({}, {}):", level, index)?;
                write!(indented(f), "{}", expr)
            }
            AstExpression::ArgWrite(level, index, expr) => {
                writeln!(f, "ArgWrite({}, {}):", level, index)?;
                write!(indented(f), "{}", expr)
            }
            AstExpression::FieldWrite(index, expr) => {
                writeln!(f, "FieldWrite({}):", index)?;
                write!(indented(f), "{}", expr)
            }
            AstExpression::Message(message) => {
                let msg = &message.message;
                writeln!(f, "Message \"{}\":", msg.signature)?;
                writeln!(indented(f), "Receiver:")?;
                write!(indented(&mut indented(f)), "{}", msg.receiver)?;
                writeln!(indented(f), "Values: {}", if msg.values.is_empty() { "(none)" } else { "" })?;
                for value in &msg.values {
                    write!(indented(&mut indented(f)), "{}", value)?;
                }
                Ok(())
            }
            AstExpression::SuperMessage(msg) => {
                writeln!(f, "SuperMessage \"{}\":", msg.signature)?;
                writeln!(indented(f), "Receiver: {} (is static: {})", msg.receiver_name, msg.is_static_class_call)?;
                writeln!(indented(f), "Values: {}", if msg.values.is_empty() { "(none)" } else { "" })?;
                for value in &msg.values {
                    write!(indented(&mut indented(f)), "{}", value)?;
                }
                Ok(())
            }
            AstExpression::BinaryOp(op) => {
                writeln!(f, "BinaryOp({})", op.op)?;
                writeln!(indented(f), "LHS:")?;
                write!(indented(&mut indented(f)), "{}", op.lhs)?;
                writeln!(indented(f), "RHS:")?;
                write!(indented(&mut indented(f)), "{}", op.rhs)
            }
            AstExpression::LocalExit(expr) => {
                writeln!(f, "LocalExit")?;
                writeln!(indented(f), "{}", expr)
            }
            AstExpression::NonLocalExit(expr, index) => {
                writeln!(f, "NonLocalExit({})", index)?;
                writeln!(indented(f), "{}", expr)
            }
            AstExpression::Literal(literal) => writeln!(f, "Literal({:?})", literal),
            AstExpression::Block(block) => {
                writeln!(f, "Block:")?;
                writeln!(indented(f), "{}", block.borrow())
            }
            AstExpression::InlinedCall(inlined_node) => match inlined_node.as_ref() {
                InlinedNode::IfInlined(node) => writeln!(f, "{}", node),
                InlinedNode::IfTrueIfFalseInlined(node) => writeln!(f, "{}", node),
                InlinedNode::WhileInlined(node) => writeln!(f, "{}", node),
                InlinedNode::OrInlined(node) => writeln!(f, "{}", node),
                InlinedNode::AndInlined(node) => writeln!(f, "{}", node),
            },
        }
    }
}
