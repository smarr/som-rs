use som_interpreter_bc::compiler;
use som_interpreter_bc::frame::Frame;
use som_interpreter_bc::method::Method;
use som_interpreter_bc::universe::{Universe, HEAP_SIZE};
use som_interpreter_bc::value::Value;
use som_lexer::{Lexer, Token};
use som_parser::lang;
use std::path::PathBuf;
use som_gc::gc_interface::GCInterface;
use som_gc::gcref::GCRef;

fn setup_universe() -> Universe {
    let classpath = vec![
        PathBuf::from("../core-lib/Smalltalk"),
        PathBuf::from("../core-lib/TestSuite/BasicInterpreterTests"),
    ];
    Universe::with_classpath(classpath, GCInterface::init(HEAP_SIZE)).expect("could not setup test universe")
}

fn get_method(method_txt: &str, method_name: &str, universe: &mut Universe) -> GCRef<Method> {
    let method_name_interned = universe.intern_symbol(method_name);

    let class_txt = format!("Foo = ( {} )", method_txt);

    let mut lexer = Lexer::new(class_txt)
        .skip_comments(true)
        .skip_whitespace(true);
    let tokens: Vec<Token> = lexer.by_ref().collect();
    assert!(lexer.text().is_empty(), "could not fully tokenize test expression");

    let class_def = som_parser::apply(lang::class_def(), tokens.as_slice()).unwrap();

    let object_class = universe.object_class();
    let class = compiler::compile_class(&mut universe.interner, &class_def, Some(&object_class), &mut universe.gc_interface);
    assert!(class.is_some(), "could not compile test expression");

    class.unwrap()
        .to_obj()
        .lookup_method(method_name_interned)
        .expect("method not found somehow?")
}

#[test]
fn frame_basic_local_access() {
    let mut universe = setup_universe();

    let method_ref = get_method("foo = ( | a b c | ^ false )", "foo", &mut universe);

    let frame = Frame::alloc_from_method(method_ref, &[], GCRef::default(), &mut universe.gc_interface);

    frame.to_obj().assign_local(0, Value::Integer(42));
    assert_eq!(frame.to_obj().lookup_local(0).as_integer(), Some(42));

    frame.to_obj().assign_local(0, Value::Integer(24));
    assert_eq!(frame.to_obj().lookup_local(0).as_integer(), Some(24));

    frame.to_obj().assign_local(0, Value::Double(400.004));
    frame.to_obj().assign_local(1, Value::NIL);

    let str_ptr = GCRef::<String>::alloc(String::from("abcd"), &mut universe.gc_interface);
    frame.to_obj().assign_local(2, Value::String(str_ptr));

    assert_eq!(frame.to_obj().lookup_local(0).as_double(), Some(400.004));
    assert_eq!(frame.to_obj().lookup_local(1), &Value::NIL);
    assert_eq!(frame.to_obj().lookup_local(2).as_string(), Some(str_ptr));
}

#[test]
fn frame_basic_arg_access() {
    let mut universe = setup_universe();

    let method_ref = get_method("foo: a and: b also: c = ( ^ false )", "foo:and:also:", &mut universe);

    let frame = Frame::alloc_from_method(method_ref, &[Value::NIL, Value::INTEGER_ZERO, Value::INTEGER_ONE], GCRef::default(), &mut universe.gc_interface);

    assert_eq!(frame.to_obj().lookup_argument(0), &Value::NIL);
    assert_eq!(frame.to_obj().lookup_argument(1), &Value::INTEGER_ZERO);
    assert_eq!(frame.to_obj().lookup_argument(2), &Value::INTEGER_ONE);

    frame.to_obj().assign_arg(2, Value::Boolean(true));
    assert_eq!(frame.to_obj().lookup_argument(2).as_boolean(), Some(true));
}

#[test]
fn frame_mixed_local_and_arg_access() {
    let mut universe = setup_universe();

    let method_ref = get_method("foo: a and: b = ( | a b c | ^ false )", "foo:and:", &mut universe);

    let frame = Frame::alloc_from_method(method_ref,
                                             &[Value::Double(1000.0), Value::SYSTEM],
                                             GCRef::default(),
                                             &mut universe.gc_interface);

    assert_eq!(frame.to_obj().lookup_argument(0), &Value::Double(1000.0));
    assert_eq!(frame.to_obj().lookup_argument(1), &Value::SYSTEM);
    assert_eq!(frame.to_obj().lookup_local(0), &Value::NIL);
    assert_eq!(frame.to_obj().lookup_local(1), &Value::NIL);
    assert_eq!(frame.to_obj().lookup_local(2), &Value::NIL);

    frame.to_obj().assign_arg(0, Value::Boolean(true));
    frame.to_obj().assign_local(0, Value::Boolean(false));

    assert_eq!(frame.to_obj().lookup_argument(0).as_boolean(), Some(true));
    assert_eq!(frame.to_obj().lookup_local(0).as_boolean(), Some(false));

    frame.to_obj().assign_arg(1, Value::Integer(42));
    frame.to_obj().assign_local(2, Value::Double(42.42));

    assert_eq!(frame.to_obj().lookup_argument(1).as_integer(), Some(42));
    assert_eq!(frame.to_obj().lookup_local(2).as_double(), Some(42.42));
}

#[test]
fn frame_stack_accesses() {
    let mut universe = setup_universe();

    let method_ref = get_method("foo: a and: b = ( | a b c | ^ false )", "foo:and:", &mut universe);

    let frame_ptr = Frame::alloc_from_method(method_ref,
                                             &[Value::Double(1000.0), Value::SYSTEM],
                                             GCRef::default(),
                                             &mut universe.gc_interface);
    let frame = frame_ptr.to_obj();
    
    assert_eq!(Frame::stack_len(frame_ptr), 0);
    frame.stack_push(Value::Boolean(true));
    assert_eq!(Frame::stack_len(frame_ptr), 1);
    
    assert_eq!(frame.stack_pop().as_boolean(), Some(true));
    assert_eq!(Frame::stack_len(frame_ptr), 0);

    frame.stack_push(Value::Integer(10000));
    frame.stack_push(Value::Double(424242.424242));
    assert_eq!(Frame::stack_len(frame_ptr), 2);
    
    assert_eq!(frame.stack_last().as_double(), Some(424242.424242));
    assert_eq!(frame.stack_last_mut().as_double(), Some(424242.424242));

    assert_eq!(frame.stack_nth_back(0).as_double(), Some(424242.424242));
    assert_eq!(frame.stack_nth_back(1).as_integer(), Some(10000));
}

#[test]
fn frame_stack_split_off() {
    let mut universe = setup_universe();

    let method_ref = get_method("foo: a and: b = ( | a b c | ^ false )", "foo:and:", &mut universe);

    let frame_ptr = Frame::alloc_from_method(method_ref,
                                             &[Value::Double(1000.0), Value::SYSTEM],
                                             GCRef::default(),
                                             &mut universe.gc_interface);
    let frame = frame_ptr.to_obj();

    frame.stack_push(Value::Integer(10000));
    frame.stack_push(Value::Double(424242.424242));
    frame.stack_push(Value::NIL);
    frame.stack_push(Value::INTEGER_ONE);
    frame.stack_push(Value::Boolean(true));

    assert_eq!(Frame::stack_len(frame_ptr), 5);
    
    let two_last = frame.stack_n_last_elements(2);

    assert_eq!(two_last, vec![Value::INTEGER_ONE, Value::Boolean(true)]);

    assert_eq!(Frame::stack_len(frame_ptr), 3);
    assert_eq!(frame.stack_last(), &Value::NIL);
}
