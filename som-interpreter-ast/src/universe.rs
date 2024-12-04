use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::gc::{get_callbacks_for_gc, VecValue};
use crate::invokable::{Invoke, Return};
use crate::value::Value;
use crate::vm_objects::block::Block;
use crate::vm_objects::class::Class;
use crate::vm_objects::frame::{Frame, FrameAccess};
use anyhow::{anyhow, Error};
use som_core::core_classes::CoreClasses;
use som_core::interner::{Interned, Interner};
use som_gc::gc_interface::GCInterface;
use som_gc::gcref::Gc;

/// GC default heap size
pub const DEFAULT_HEAP_SIZE: usize = 1024 * 1024 * 256;

/// The central data structure for the interpreter.
///
/// It represents the complete state of the interpreter, like the known class definitions,
/// the string interner and the stack frames.
pub struct Universe {
    /// The string interner for symbols.
    pub interner: Interner,
    /// The known global bindings.
    pub globals: HashMap<String, Value>,
    /// The path to search in for new classes.
    pub classpath: Vec<PathBuf>,
    /// The current frame for the operation
    pub current_frame: Gc<Frame>,
    /// The interpreter's core classes.
    pub core: CoreClasses<Gc<Class>>,
    /// The time record of the universe's creation.
    pub start_time: Instant,
    /// GC interface
    pub gc_interface: &'static mut GCInterface,

    // we could pass arguments using the Rust stack, and we used to: but with moving GC, that makes them often unreachable, so we need to manage our own stack
    pub stack_args: Vec<Value>,
}

impl Drop for Universe {
    fn drop(&mut self) {
        let _box: Box<GCInterface> = unsafe { Box::from_raw(self.gc_interface) };
        drop(_box)
    }
}

impl Universe {
    /// Initialize the universe from the given classpath.
    pub fn with_classpath(classpath: Vec<PathBuf>) -> Result<Self, Error> {
        Self::with_classpath_and_heap_size(classpath, DEFAULT_HEAP_SIZE)
    }

    /// Initialize the universe from the given classpath, and given a heap size
    pub fn with_classpath_and_heap_size(classpath: Vec<PathBuf>, heap_size: usize) -> Result<Self, Error> {
        let interner = Interner::with_capacity(100);
        let mut globals = HashMap::new();

        let gc_interface = GCInterface::init(heap_size, get_callbacks_for_gc());

        let object_class = Self::load_system_class(classpath.as_slice(), "Object", None, gc_interface)?;
        let mut class_class = Self::load_system_class(classpath.as_slice(), "Class", Some(object_class), gc_interface)?;
        let metaclass_class = Self::load_system_class(classpath.as_slice(), "Metaclass", Some(class_class), gc_interface)?;

        let mut nil_class = Self::load_system_class(classpath.as_slice(), "Nil", Some(object_class), gc_interface)?;
        let mut integer_class = Self::load_system_class(classpath.as_slice(), "Integer", Some(object_class), gc_interface)?;
        let mut array_class = Self::load_system_class(classpath.as_slice(), "Array", Some(object_class), gc_interface)?;
        let mut method_class = Self::load_system_class(classpath.as_slice(), "Method", Some(object_class), gc_interface)?; // was array_class in original code?
        let mut string_class = Self::load_system_class(classpath.as_slice(), "String", Some(object_class), gc_interface)?;
        let mut symbol_class = Self::load_system_class(classpath.as_slice(), "Symbol", Some(string_class), gc_interface)?;
        let mut primitive_class = Self::load_system_class(classpath.as_slice(), "Primitive", Some(object_class), gc_interface)?;
        let mut system_class = Self::load_system_class(classpath.as_slice(), "System", Some(object_class), gc_interface)?;
        let mut double_class = Self::load_system_class(classpath.as_slice(), "Double", Some(object_class), gc_interface)?;

        let mut block_class = Self::load_system_class(classpath.as_slice(), "Block", Some(object_class), gc_interface)?;
        let mut block1_class = Self::load_system_class(classpath.as_slice(), "Block1", Some(block_class), gc_interface)?;
        let mut block2_class = Self::load_system_class(classpath.as_slice(), "Block2", Some(block_class), gc_interface)?;
        let mut block3_class = Self::load_system_class(classpath.as_slice(), "Block3", Some(block_class), gc_interface)?;

        let mut boolean_class = Self::load_system_class(classpath.as_slice(), "Boolean", Some(object_class), gc_interface)?;
        let mut true_class = Self::load_system_class(classpath.as_slice(), "True", Some(boolean_class), gc_interface)?;
        let mut false_class = Self::load_system_class(classpath.as_slice(), "False", Some(boolean_class), gc_interface)?;

        // initializeSystemClass(objectClass, null, "Object");
        // set_super_class(&object_class, &nil_class, &metaclass_class);
        object_class.class().set_class(&metaclass_class);
        object_class.class().set_super_class(&class_class);

        // initializeSystemClass(classClass, objectClass, "Class");
        set_super_class(&mut class_class, &object_class, &metaclass_class);
        // initializeSystemClass(metaclassClass, classClass, "Metaclass");
        set_super_class(&mut metaclass_class.clone(), &class_class, &metaclass_class);
        // initializeSystemClass(nilClass, objectClass, "Nil");
        set_super_class(&mut nil_class, &object_class, &metaclass_class);
        // initializeSystemClass(arrayClass, objectClass, "Array");
        set_super_class(&mut array_class, &object_class, &metaclass_class);
        // initializeSystemClass(methodClass, arrayClass, "Method");
        set_super_class(&mut method_class, &array_class, &metaclass_class);
        // initializeSystemClass(stringClass, objectClass, "String");
        set_super_class(&mut string_class, &object_class, &metaclass_class);
        // initializeSystemClass(symbolClass, stringClass, "Symbol");
        set_super_class(&mut symbol_class, &string_class, &metaclass_class);
        // initializeSystemClass(integerClass, objectClass, "Integer");
        set_super_class(&mut integer_class, &object_class, &metaclass_class);
        // initializeSystemClass(primitiveClass, objectClass, "Primitive");
        set_super_class(&mut primitive_class, &object_class, &metaclass_class);
        // initializeSystemClass(doubleClass, objectClass, "Double");
        set_super_class(&mut double_class, &object_class, &metaclass_class);

        set_super_class(&mut system_class, &object_class, &metaclass_class);

        set_super_class(&mut block_class, &object_class, &metaclass_class);
        set_super_class(&mut block1_class, &block_class, &metaclass_class);
        set_super_class(&mut block2_class, &block_class, &metaclass_class);
        set_super_class(&mut block3_class, &block_class, &metaclass_class);

        set_super_class(&mut boolean_class, &object_class, &metaclass_class);
        set_super_class(&mut true_class, &boolean_class, &metaclass_class);
        set_super_class(&mut false_class, &boolean_class, &metaclass_class);

        globals.insert("Object".into(), Value::Class(object_class));
        globals.insert("Class".into(), Value::Class(class_class));
        globals.insert("Metaclass".into(), Value::Class(metaclass_class));
        globals.insert("Nil".into(), Value::Class(nil_class));
        globals.insert("Integer".into(), Value::Class(integer_class));
        globals.insert("Array".into(), Value::Class(array_class));
        globals.insert("Method".into(), Value::Class(method_class));
        globals.insert("Symbol".into(), Value::Class(symbol_class));
        globals.insert("Primitive".into(), Value::Class(primitive_class));
        globals.insert("String".into(), Value::Class(string_class));
        globals.insert("System".into(), Value::Class(system_class));
        globals.insert("Double".into(), Value::Class(double_class));
        globals.insert("Boolean".into(), Value::Class(boolean_class));
        globals.insert("True".into(), Value::Class(true_class));
        globals.insert("False".into(), Value::Class(false_class));
        globals.insert("Block".into(), Value::Class(block_class));
        globals.insert("Block1".into(), Value::Class(block1_class));
        globals.insert("Block2".into(), Value::Class(block2_class));
        globals.insert("Block3".into(), Value::Class(block3_class));

        globals.insert("true".into(), Value::Boolean(true));
        globals.insert("false".into(), Value::Boolean(false));
        globals.insert("nil".into(), Value::NIL);
        globals.insert("system".into(), Value::SYSTEM);

        Ok(Self {
            globals,
            interner,
            classpath,
            current_frame: Gc::default(),
            start_time: Instant::now(),
            core: CoreClasses {
                object_class,
                class_class,
                metaclass_class,
                nil_class,
                integer_class,
                array_class,
                method_class,
                symbol_class,
                primitive_class,
                string_class,
                system_class,
                double_class,
                block_class,
                block1_class,
                block2_class,
                block3_class,
                boolean_class,
                true_class,
                false_class,
            },
            gc_interface,
            stack_args: vec![],
        })
    }

    /// Load a class from its name into this universe.
    pub fn load_class(&mut self, class_name: impl Into<String>) -> Result<Gc<Class>, Error> {
        let class_name = class_name.into();

        for path in &self.classpath {
            let mut path = path.join(class_name.as_str());
            path.set_extension("som");

            // Read file contents.
            let contents = match fs::read_to_string(path.as_path()) {
                Ok(contents) => contents,
                Err(_) => continue,
            };

            // Collect all tokens from the file.
            let tokens: Vec<_> = som_lexer::Lexer::new(contents.as_str()).skip_comments(true).skip_whitespace(true).collect();

            // Parse class definition from the tokens.
            let defn = match som_parser::parse_file(tokens.as_slice()) {
                Some(defn) => defn,
                None => continue,
            };

            if defn.name != class_name {
                return Err(anyhow!("{}: class name is different from file name.", path.display(),));
            }

            let super_class = if let Some(ref super_class) = defn.super_class {
                self.lookup_global(super_class).and_then(Value::as_class).unwrap_or_else(|| self.load_class(super_class).unwrap())
            } else {
                self.core.object_class
            };

            let mut class = Class::from_class_def(defn, Some(super_class), self.gc_interface).map_err(Error::msg)?;
            set_super_class(&mut class, &super_class, &self.core.metaclass_class);

            /*fn has_duplicated_field(class: &SOMRef<Class>) -> Option<(String, (String, String))> {
                let super_class_iterator = std::iter::successors(Some(class), |class| {
                    class.borrow().super_class()
                });
                let mut map = HashMap::<String, String>::new();
                for class in super_class_iterator {
                    let class_name = class.borrow().name().to_string();
                    for (field, _) in class.borrow().locals.iter() {
                        let field_name = field;
                        match map.entry(field_name) {
                            Entry::Occupied(entry) => {
                                return Some((field_name, (class_name, entry.get())))
                            }
                            Entry::Vacant(v) => {
                                v.insert(class_name);
                            }
                        }
                    }
                }
                return None;
            }*/

            /*if let Some((field, (c1, c2))) = has_duplicated_field(&class) {
                return Err(anyhow!(
                    "the field named '{}' is defined more than once (by '{}' and '{}', where the latter inherits from the former)",
                    field, c1, c2,
                ));
            }

            if let Some((field, (c1, c2))) = has_duplicated_field(&class.borrow().class()) {
                return Err(anyhow!(
                    "the field named '{}' is defined more than once (by '{}' and '{}', where the latter inherits from the former)",
                    field, c1, c2,
                ));
            }*/

            self.globals.insert(class.name().to_string(), Value::Class(class));

            return Ok(class);
        }

        Err(anyhow!("could not find the '{}' class", class_name))
    }

    /// Load a system class (with an incomplete hierarchy).
    pub fn load_system_class(
        classpath: &[impl AsRef<Path>],
        class_name: impl Into<String>,
        super_class: Option<Gc<Class>>,
        gc_interface: &mut GCInterface,
    ) -> Result<Gc<Class>, Error> {
        let class_name = class_name.into();
        for path in classpath {
            let mut path = path.as_ref().join(class_name.as_str());
            path.set_extension("som");

            // Read file contents.
            let contents = match fs::read_to_string(path.as_path()) {
                Ok(contents) => contents,
                Err(err) if err.kind() == io::ErrorKind::NotFound => continue,
                Err(err) => return Err(Error::from(err)),
            };

            // Collect all tokens from the file.
            let tokens: Vec<_> = som_lexer::Lexer::new(contents.as_str()).skip_comments(true).skip_whitespace(true).collect();

            // Parse class definition from the tokens.
            let defn = match som_parser::parse_file_no_universe(tokens.as_slice()) {
                Some(defn) => defn,
                None => return Err(anyhow!("could not parse the '{}' system class", class_name)),
            };

            if defn.name != class_name {
                return Err(anyhow!("{}: class name is different from file name.", path.display(),));
            }

            return Class::from_class_def(defn, super_class, gc_interface).map_err(Error::msg);
        }

        Err(anyhow!("could not find the '{}' system class", class_name))
    }

    /// Get the **Object** class.
    pub fn object_class(&self) -> Gc<Class> {
        self.core.object_class
    }

    /// Get the **Nil** class.
    pub fn nil_class(&self) -> Gc<Class> {
        self.core.nil_class
    }
    /// Get the **System** class.
    pub fn system_class(&self) -> Gc<Class> {
        self.core.system_class
    }

    /// Get the **Symbol** class.
    pub fn symbol_class(&self) -> Gc<Class> {
        self.core.symbol_class
    }
    /// Get the **String** class.
    pub fn string_class(&self) -> Gc<Class> {
        self.core.string_class
    }
    /// Get the **Array** class.
    pub fn array_class(&self) -> Gc<Class> {
        self.core.array_class
    }

    /// Get the **Integer** class.
    pub fn integer_class(&self) -> Gc<Class> {
        self.core.integer_class
    }
    /// Get the **Double** class.
    pub fn double_class(&self) -> Gc<Class> {
        self.core.double_class
    }

    /// Get the **Block** class.
    pub fn block_class(&self) -> Gc<Class> {
        self.core.block_class
    }
    /// Get the **Block1** class.
    pub fn block1_class(&self) -> Gc<Class> {
        self.core.block1_class
    }
    /// Get the **Block2** class.
    pub fn block2_class(&self) -> Gc<Class> {
        self.core.block2_class
    }
    /// Get the **Block3** class.
    pub fn block3_class(&self) -> Gc<Class> {
        self.core.block3_class
    }

    /// Get the **True** class.
    pub fn true_class(&self) -> Gc<Class> {
        self.core.true_class
    }
    /// Get the **False** class.
    pub fn false_class(&self) -> Gc<Class> {
        self.core.false_class
    }

    /// Get the **Metaclass** class.
    pub fn metaclass_class(&self) -> Gc<Class> {
        self.core.metaclass_class
    }

    /// Get the **Method** class.
    pub fn method_class(&self) -> Gc<Class> {
        self.core.method_class
    }
    /// Get the **Primitive** class.
    pub fn primitive_class(&self) -> Gc<Class> {
        self.core.primitive_class
    }
}

impl Universe {
    pub fn with_frame<T>(&mut self, nbr_locals: u8, nbr_args: usize, func: impl FnOnce(&mut Self) -> T) -> T {
        let frame = Frame::alloc_new_frame(nbr_locals, nbr_args, self);
        self.current_frame = frame;
        let ret = func(self);
        self.current_frame = self.current_frame.prev_frame;
        ret
    }

    /// Intern a symbol.
    pub fn intern_symbol(&mut self, symbol: &str) -> Interned {
        self.interner.intern(symbol)
    }

    /// Lookup a symbol.
    pub fn lookup_symbol(&self, symbol: Interned) -> &str {
        self.interner.lookup(symbol)
    }

    /// Search for a local binding.
    pub fn lookup_local(&self, idx: u8) -> &Value {
        self.current_frame.lookup_local(idx)
    }

    /// Look up a variable we know to have been defined in another scope.
    pub fn lookup_non_local(&self, idx: u8, target_scope: u8) -> Value {
        *Frame::nth_frame_back(&self.current_frame, target_scope).lookup_local(idx)
    }

    /// Look up a field.
    pub fn lookup_field(&self, idx: u8) -> Value {
        self.current_frame.lookup_field(idx)
    }

    pub fn lookup_arg(&self, idx: u8, scope: u8) -> Value {
        *Frame::nth_frame_back(&self.current_frame, scope).lookup_argument(idx)
    }

    /// Returns whether a global binding of the specified name exists.
    pub fn has_global(&self, name: impl AsRef<str>) -> bool {
        let name = name.as_ref();
        self.globals.contains_key(name)
    }

    /// Search for a global binding.
    pub fn lookup_global(&self, name: impl AsRef<str>) -> Option<Value> {
        let name = name.as_ref();
        self.globals.get(name).cloned()
    }

    /// Assign a value to a local binding.
    pub fn assign_local(&mut self, idx: u8, value: &Value) {
        self.current_frame.assign_local(idx, *value)
    }

    pub fn assign_non_local(&mut self, idx: u8, scope: u8, value: &Value) {
        Frame::nth_frame_back(&self.current_frame, scope).assign_local(idx, *value)
    }

    pub fn assign_field(&mut self, idx: u8, value: &Value) {
        // dbg!(&idx);
        self.current_frame.assign_field(idx, value)
    }

    pub fn assign_arg(&mut self, idx: u8, scope: u8, value: &Value) {
        Frame::nth_frame_back(&self.current_frame, scope).assign_arg(idx, *value)
    }

    /// Assign a value to a global binding.
    pub fn assign_global(&mut self, name: impl AsRef<str>, value: &Value) -> Option<()> {
        self.globals.insert(name.as_ref().to_string(), *value).map(|_| ())
    }

    #[inline(always)]
    /// Remove N elements off the argument stack and return them as their own vector
    pub fn stack_n_last_elems(&mut self, n: usize) -> Vec<Value> {
        let idx_split_off = self.stack_args.len() - n;
        self.stack_args.split_off(idx_split_off)
    }
}

impl Universe {
    /// Call `escapedBlock:` on the given value, if it is defined.
    pub fn escaped_block(&mut self, value: Value, block: Gc<Block>) -> Option<Return> {
        let mut initialize = value.lookup_method(self, "escapedBlock:")?;

        self.stack_args.push(value);
        self.stack_args.push(Value::Block(block));
        let escaped_block_result = initialize.invoke(self, 2);
        Some(escaped_block_result)
    }

    /// Call `doesNotUnderstand:` on the given value, if it is defined.
    pub fn does_not_understand(&mut self, value: Value, symbol: impl AsRef<str>, args: Vec<Value>) -> Option<Return> {
        let mut initialize = value.lookup_method(self, "doesNotUnderstand:arguments:")?;
        let sym = self.intern_symbol(symbol.as_ref());
        let sym = Value::Symbol(sym);
        let args = Value::Array(self.gc_interface.alloc(VecValue(args)));

        // eprintln!("Couldn't invoke {}; exiting.", symbol.as_ref()); std::process::exit(1);

        self.stack_args.push(value);
        self.stack_args.push(sym);
        self.stack_args.push(args);

        let dnu_result = initialize.invoke(self, 3);
        Some(dnu_result)
    }

    /// Call `unknownGlobal:` on the given value, if it is defined.
    pub fn unknown_global(&mut self, value: Value, name: impl AsRef<str>) -> Option<Return> {
        let sym = self.intern_symbol(name.as_ref());
        let mut method = value.lookup_method(self, "unknownGlobal:")?;

        self.stack_args.push(value);
        self.stack_args.push(Value::Symbol(sym));

        let unknown_global_result = method.invoke(self, 2);
        match unknown_global_result {
            Return::Local(value) | Return::NonLocal(value, _) => Some(Return::Local(value)),
            #[cfg(feature = "inlining-disabled")]
            Return::Restart => panic!("(from 'System>>#unknownGlobal:') incorrectly asked for a restart"),
        }
    }

    /// Call `System>>#initialize:` with the given name, if it is defined.
    pub fn initialize(&mut self, args: Vec<Value>) -> Option<Return> {
        let mut initialize = Value::SYSTEM.lookup_method(self, "initialize:")?;
        let args = Value::Array(self.gc_interface.alloc(VecValue(args)));
        self.stack_args.push(Value::SYSTEM);
        self.stack_args.push(args);
        let program_result = initialize.invoke(self, 2);
        Some(program_result)
    }
}

fn set_super_class(class: &mut Gc<Class>, super_class: &Gc<Class>, metaclass_class: &Gc<Class>) {
    class.set_super_class(super_class);

    class.class().set_super_class(&super_class.class());
    class.class().set_class(metaclass_class);
}
