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
    UnaryDispatch(Box<AstUnaryDispatch>),
    BinaryDispatch(Box<AstBinaryDispatch>),
    TernaryDispatch(Box<AstTernaryDispatch>),
    NAryDispatch(Box<AstNAryDispatch>),
    SuperMessage(Box<AstSuperMessage>),
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

pub type CacheEntry = (*const Class, SOMRef<Method>);

#[derive(Debug, Clone, PartialEq)]
pub struct AstDispatchNode {
    pub signature: String,
    pub receiver: AstExpression,
    pub inline_cache: Option<CacheEntry>
}

#[derive(Debug, Clone, PartialEq)]
pub struct AstUnaryDispatch {
    pub dispatch_node: AstDispatchNode
}

#[derive(Debug, Clone, PartialEq)]
pub struct AstBinaryDispatch {
    pub dispatch_node: AstDispatchNode,
    pub arg: AstExpression,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AstTernaryDispatch {
    pub dispatch_node: AstDispatchNode,
    pub arg1: AstExpression,
    pub arg2: AstExpression,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AstNAryDispatch {
    pub dispatch_node: AstDispatchNode,
    pub values: Vec<AstExpression>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AstSuperMessage {
    pub super_class: SOMRef<Class>,
    pub signature: String,
    pub values: Vec<AstExpression>,
    // NB: no inline cache. I don't think it's super worth it since super calls are uncommon. Easy to implement though
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
            AstExpression::UnaryDispatch(op) => {
                writeln!(f, "UnaryDispatch({})", op.dispatch_node.signature)?;
                writeln!(indented(f), "Receiver:")?;
                write!(indented(&mut indented(f)), "{}", op.dispatch_node.receiver)
            }
            AstExpression::BinaryDispatch(op) => {
                writeln!(f, "BinaryDispatch({})", op.dispatch_node.signature)?;
                writeln!(indented(f), "Receiver:")?;
                write!(indented(&mut indented(f)), "{}", op.dispatch_node.receiver)?;
                writeln!(indented(f), "arg:")?;
                write!(indented(&mut indented(f)), "{}", op.arg)
            }
            AstExpression::TernaryDispatch(op) => {
                writeln!(f, "TernaryDispatch({})", op.dispatch_node.signature)?;
                writeln!(indented(f), "Receiver:")?;
                write!(indented(&mut indented(f)), "{}", op.dispatch_node.receiver)?;
                writeln!(indented(f), "arg1:")?;
                write!(indented(&mut indented(f)), "{}", op.arg1)?;
                writeln!(indented(f), "arg2:")?;
                write!(indented(&mut indented(f)), "{}", op.arg2)
            }
            AstExpression::NAryDispatch(msg) => {
                writeln!(f, "Message \"{}\":", msg.dispatch_node.signature)?;
                writeln!(indented(f), "Receiver:")?;
                write!(indented(&mut indented(f)), "{}", msg.dispatch_node.receiver)?;
                writeln!(indented(f), "Values: {}", if msg.values.is_empty() { "(none)" } else { "" })?;
                for value in &msg.values {
                    write!(indented(&mut indented(f)), "{}", value)?;
                }
                Ok(())
            }
            AstExpression::SuperMessage(msg) => {
                writeln!(f, "SuperMessage \"{}\":", msg.signature)?;
                writeln!(indented(f), "Receiver: {}", msg.super_class.borrow().name)?;
                writeln!(indented(f), "Values: {}", if msg.values.is_empty() { "(none)" } else { "" })?;
                for value in &msg.values {
                    write!(indented(&mut indented(f)), "{}", value)?;
                }
                Ok(())
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
