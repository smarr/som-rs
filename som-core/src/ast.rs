/// Represents a class definition.
///
/// Example:
/// ```text
/// Counter = (
///   | total |
///   new = ( self reset )
///   increment = ( total := total + 1 )
///   get = ( ^ total )
///   reset = ( total := 0 )
/// )
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ClassDef {
    /// The name of the class.
    pub name: String,
    /// The name of the superclass.
    pub super_class: Option<String>,
    /// The locals for instances of that class.
    pub instance_locals: Vec<String>,
    /// The methods declared for instances of that class.
    pub instance_methods: Vec<MethodDef>,
    /// The static locals for that class.
    pub static_locals: Vec<String>,
    /// The static methods declared for that class.
    pub static_methods: Vec<MethodDef>,
}

/// Represents a method definition.
///
/// Example:
/// ```text
/// "unary method"       increment = ( self increment: 1 )
/// "positional method"  increment: value = ( total := total + value )
/// "operator method"    + value = ( self increment: value )
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct MethodDef {
    /// The method's signature (eg. `println`, `at:put:` or `==`).
    pub signature: String,
    /// The method's body.
    pub body: MethodBody,
}

/// Represents a method's body.
///
/// Exemple:
/// ```text
/// "primitive method body"
/// printString: string = primitive
///
/// "actual method body, with a local"
/// double: value = ( |clone| clone := double. ^ (double + clone) )
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum MethodBody {
    /// A primitive (meant to be implemented by the VM itself).
    Primitive,
    /// An actual body for the method, with locals.
    Body {
        locals_nbr: usize,
        body: Body,
        #[cfg(feature = "block-debug-info")]
        debug_info: BlockDebugInfo,
    },
}

/// Represents the contents of a body (within a term or block).
///
/// Exemple:
/// ```text
/// "body within a term"
/// new = (
///     local := counter + 5.
///     (counter get) > 5
/// )
///
/// "body within a block"
/// [ :arg |
///     local := counter + arg.
///     arg * (counter get)
/// ]
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Body {
    /// The expressions in the body.
    pub exprs: Vec<Expression>,
    /// Is the last expression terminated with a period ?
    pub full_stopped: bool,
}

/// Represents an expression.
///
/// Exemple:
/// ```text
/// "reference"          counter
/// "assignment"         counter := 10
/// "messsage send"      counter incrementBy: 5
/// "binary operation"   counter <= 5
/// "exit operation"     ^counter
/// "literal"            'foo'
/// "block"              [ :value | counter incrementBy: value ]
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    /// A reference to a binding (eg. `counter`).
    GlobalRead(String),
    /// This does NOT exist: this is a field write which will be resolved by the AST/BC compilers, or it's an error.
    GlobalWrite(String, Box<Expression>),
    /// Read of a local var.
    LocalVarRead(usize),
    /// Read of a nonlocal var.
    NonLocalVarRead(usize, usize),
    /// Read of an argument.
    ArgRead(usize, usize),
    /// An assignment to a binding (eg. `counter := 10`).
    LocalVarWrite(usize, Box<Expression>),
    NonLocalVarWrite(usize, usize, Box<Expression>),
    ArgWrite(usize, usize, Box<Expression>),
    /// A message send (eg. `counter incrementBy: 5`).
    Message(Box<Message>),
    /// An exit operation (eg. `^counter`). Second argument is the scope level to differentiate local and nonlocal returns
    Exit(Box<Expression>, usize),
    /// A literal (eg. `'foo'`, `10`, `#foo`, ...).
    Literal(Literal),
    /// A block (eg. `[ :value | counter incrementBy: value ]`).
    Block(Block),
}

/// Represents a message send.
///
/// Exemple:
/// ```text
/// "unary message send"
/// 'hello, world' println
///
/// "positional message send"
/// range from: 0 to: 10
///
/// "binary operator message send"
/// value == 3
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    /// The object to which the message is sent to.
    pub receiver: Expression,
    /// The signature of the message (eg. "ifTrue:ifFalse:").
    pub signature: String,
    /// The list of dynamic values that are passed.
    pub values: Vec<Expression>,
}

/// A message with "super" as the receiver, so the superclass.
#[derive(Debug, Clone, PartialEq)]
pub struct SuperMessage {
    /// The object to which the message is sent to. TODO should not do a super call, but cache the class, really.
    pub receiver_name: String,
    /// Do we access the static or instance methods of the superclass?
    pub is_static_class_call: bool,
    /// The signature of the message (eg. "ifTrue:ifFalse:").
    pub signature: String,
    /// The list of dynamic values that are passed.
    pub values: Vec<Expression>,
}

/// Represents a binary operation.
///
/// Exemple:
/// ```text
/// counter <= 2
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct BinaryOp {
    /// Represents the operator symbol.
    pub op: String,
    /// Represents the left-hand side.
    pub lhs: Expression,
    /// Represents the right-hand side.
    pub rhs: Expression,
}

/// Represents a block.
///
/// Exemple:
/// ```text
/// "simple block"
/// [ 'hello, world' println ]
///
/// "block with parameter"
/// [ :value | value * 2 ]
///
/// "block with parameter and local"
/// [ :value | |serialized| serialized := value asString. serialized println ]
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    /// Represents the parameters' names.
    pub nbr_params: usize,
    /// The names of the locals.
    pub nbr_locals: usize,
    /// Represents the block's body.
    pub body: Body,
    #[cfg(feature = "block-debug-info")]
    /// Debug info for the block: parameters and local variable names
    pub dbg_info: BlockDebugInfo,
}

#[cfg(feature = "block-debug-info")]
#[derive(Debug, Clone, PartialEq)]
pub struct BlockDebugInfo {
    pub parameters: Vec<String>,
    pub locals: Vec<String>,
}

/// Represents a term.
///
/// Exemple:
/// ```text
/// "simple term"
/// ( 1 + 1 )
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Term {
    /// The body of the term.
    pub body: Body,
}

/// Represents a literal.
///
/// Exemple:
/// ```text
/// #foo     "symbol literal"
/// 'hello'  "string literal"
/// 3.14     "double literal"
/// 42       "integer literal"
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    /// Represents a symbol literal (eg. `#foo`).
    Symbol(String), // todo: in a perfect world, this would be 'static str. that requires we store source code as static though, which sounds like a lot of refactoring.
    /// Represents a string literal (eg. `'hello'`).
    String(String), // todo: ditto, maybe.
    /// Represents a decimal number literal (eg. `3.14`).
    Double(f64),
    /// Represents a integer number literal (eg. `42`).
    Integer(i32),
    /// Represents a big integer (bigger than a 64-bit signed integer can represent).
    BigInteger(String),
    /// Represents an array literal (eg. `$(1 2 3)`)
    Array(Vec<Literal>),
}
