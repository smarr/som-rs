use som_gc::gc_interface::GCInterface;
use som_interpreter_ast::ast::AstExpression::*;
use som_interpreter_ast::ast::InlinedNode::IfInlined;
use som_interpreter_ast::ast::{AstBinaryDispatch, AstBody, AstDispatchNode, AstMethodDef};
use som_interpreter_ast::compiler::AstMethodCompilerCtxt;
use som_interpreter_ast::specialized::inlined::if_inlined_node::IfInlinedNode;
use som_interpreter_ast::universe::HEAP_SIZE;
use som_lexer::{Lexer, Token};
use som_parser::lang;

fn get_ast(class_txt: &str) -> AstMethodDef {
    let mut lexer = Lexer::new(class_txt)
        .skip_comments(true)
        .skip_whitespace(true);
    let tokens: Vec<Token> = lexer.by_ref().collect();
    assert!(lexer.text().is_empty(), "could not fully tokenize test expression");

    let method_def = som_parser::apply(lang::instance_method_def(), tokens.as_slice()).unwrap();
    
    AstMethodCompilerCtxt::parse_method_def(&method_def, None, &mut GCInterface::init(HEAP_SIZE))
}

#[test]
fn if_true_inlining_ok() {
    let very_basic = "run = (
        true ifTrue: [ ^true ].
        ^ false
    )";

    let ast = get_ast(very_basic);

    assert_eq!(ast, AstMethodDef {
        signature: "run".to_string(),
        locals_nbr: 0,
        body: AstBody {
            exprs: vec![
                InlinedCall(
                    Box::new(IfInlined(
                        IfInlinedNode {
                            expected_bool: true,
                            cond_expr: GlobalRead(Box::new("true".to_string())),
                            body_instrs: AstBody {
                                exprs: vec![LocalExit(Box::new(GlobalRead(Box::new("true".to_string()))))],
                            },
                        },
                    ),
                    )),
                LocalExit(Box::new(GlobalRead(Box::new("false".to_string())))),
            ],
        },
    }
    );
}

#[test]
fn if_false_inlining_ok() {
    // based on the method of the same name defined in System
    let method_txt2 = "resolve: a = (
        | class |
        (class == nil) ifFalse: [
            ^class ].
    )";

    let resolve = get_ast(method_txt2);

    assert_eq!(resolve, AstMethodDef{
        signature: "resolve:".to_string(),
        locals_nbr: 1,
        body: AstBody {
            exprs: vec![
                InlinedCall(
                    Box::from(IfInlined(
                        IfInlinedNode {
                            expected_bool: false,
                            cond_expr: BinaryDispatch(
                                Box::new(AstBinaryDispatch {
                                    dispatch_node: AstDispatchNode {
                                        signature: "==".to_string(),
                                        receiver: LocalVarRead(0),
                                        inline_cache: None
                                    },
                                    arg: GlobalRead(Box::new("nil".to_string())),
                                }),
                            ),
                            body_instrs: AstBody { exprs: vec![LocalExit(Box::new(LocalVarRead(0)))] },
                        },
                    )),
                ),
            ],
        },
    });
}

#[test]
pub fn recursive_inlining() {
    // from Hashtable.
    let contains_key_txt = "containsKey: key = ( 
        | idx e | 
        e isNil ifFalse: [ 
            e keys do: 
                [ :k | 
                    k = key ifTrue: [ 
                        ^true 
                    ] 
                ] 
        ]. 
        )";

    let ast_answer = "Method containsKey: (2 locals):
        AstBody:
            IfInlinedNode (expected bool: false):
                condition expr:\
                    UnaryDispatch \"isNil\":
                        Receiver:
                            LocalVarRead(1)
                body block:
                    AstBody:
                        BinaryDispatch \"do:\":
                            Receiver:
                                UnaryDispatch \"keys\":
                                    Receiver:
                                        LocalVarRead(1)
                            arg:
                                Block:
                                    AstBlock(1 params, 0 locals):
                                        IfInlinedNode (expected bool: true):
                                            condition expr:
                                                BinaryDispatch \"=\":
                                                    Receiver:
                                                        ArgRead(0, 1)
                                                    arg:
                                                        ArgRead(1, 1)
                                            body block:
                                                AstBody:
                                                    NonLocalExit(1)
                                                        GlobalRead(true)";

    let resolve = get_ast(contains_key_txt);

    let cleaned_ast_answer: String = ast_answer.chars().filter(|c| !c.is_whitespace()).collect();
    let cleaned_resolve: String = resolve.to_string().chars().filter(|c| !c.is_whitespace()).collect();

    assert_eq!(cleaned_ast_answer, cleaned_resolve);
}