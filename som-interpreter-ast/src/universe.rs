use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::block::Block;
use crate::class::Class;
use crate::frame::{Frame, FrameAccess};
use crate::invokable::{Invoke, Return};
use crate::value::Value;
use anyhow::{anyhow, Error};
use som_core::gc::{GCInterface, GCRef};
use som_core::interner::{Interned, Interner};

/// The core classes of the SOM interpreter.
///
/// This struct allows to always keep a reference to important classes,
/// even in case of modifications to global bindings by user-defined code.
#[derive(Debug)]
pub struct CoreClasses {
    /// The **Object** class.
    pub object_class: GCRef<Class>,
    /// The **Class** class.
    pub class_class: GCRef<Class>,
    /// The **Class** class.
    pub metaclass_class: GCRef<Class>,

    /// The **Nil** class.
    pub nil_class: GCRef<Class>,
    /// The **Integer** class.
    pub integer_class: GCRef<Class>,
    /// The **Double** class.
    pub double_class: GCRef<Class>,
    /// The **Array** class.
    pub array_class: GCRef<Class>,
    /// The **Method** class.
    pub method_class: GCRef<Class>,
    /// The **Primitive** class.
    pub primitive_class: GCRef<Class>,
    /// The **Symbol** class.
    pub symbol_class: GCRef<Class>,
    /// The **String** class.
    pub string_class: GCRef<Class>,
    /// The **System** class.
    pub system_class: GCRef<Class>,

    /// The **Block** class.
    pub block_class: GCRef<Class>,
    /// The **Block1** class.
    pub block1_class: GCRef<Class>,
    /// The **Block2** class.
    pub block2_class: GCRef<Class>,
    /// The **Block3** class.
    pub block3_class: GCRef<Class>,

    /// The **Boolean** class.
    pub boolean_class: GCRef<Class>,
    /// The **True** class.
    pub true_class: GCRef<Class>,
    /// The **False** class.
    pub false_class: GCRef<Class>,
}

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
    pub current_frame: GCRef<Frame>,
    /// The interpreter's core classes.
    pub core: CoreClasses,
    /// The time record of the universe's creation.
    pub start_time: Instant,
    /// GC interface
    pub gc_interface: GCInterface
}

impl Universe {
    /// Initialize the universe from the given classpath.
    pub fn with_classpath(classpath: Vec<PathBuf>, mut gc_interface: GCInterface) -> Result<Self, Error> {
        let interner = Interner::with_capacity(100);
        let mut globals = HashMap::new();

        let object_class = Self::load_system_class(classpath.as_slice(), "Object", None, &mut gc_interface)?;
        let class_class = Self::load_system_class(classpath.as_slice(), "Class", Some(object_class.clone()), &mut gc_interface)?;
        let metaclass_class = Self::load_system_class(classpath.as_slice(), "Metaclass", Some(class_class.clone()), &mut gc_interface)?;

        let nil_class = Self::load_system_class(classpath.as_slice(), "Nil", Some(object_class.clone()), &mut gc_interface)?;
        let integer_class = Self::load_system_class(classpath.as_slice(), "Integer", Some(object_class.clone()), &mut gc_interface)?;
        let array_class = Self::load_system_class(classpath.as_slice(), "Array", Some(object_class.clone()), &mut gc_interface)?;
        let method_class = Self::load_system_class(classpath.as_slice(), "Method", Some(object_class.clone()), &mut gc_interface)?; // was array_class in original code?
        let string_class = Self::load_system_class(classpath.as_slice(), "String", Some(object_class.clone()), &mut gc_interface)?;
        let symbol_class = Self::load_system_class(classpath.as_slice(), "Symbol", Some(string_class.clone()), &mut gc_interface)?;
        let primitive_class = Self::load_system_class(classpath.as_slice(), "Primitive", Some(object_class.clone()), &mut gc_interface)?;
        let system_class = Self::load_system_class(classpath.as_slice(), "System", Some(object_class.clone()), &mut gc_interface)?;
        let double_class = Self::load_system_class(classpath.as_slice(), "Double", Some(object_class.clone()), &mut gc_interface)?;

        let block_class = Self::load_system_class(classpath.as_slice(), "Block", Some(object_class.clone()), &mut gc_interface)?;
        let block1_class = Self::load_system_class(classpath.as_slice(), "Block1", Some(block_class.clone()), &mut gc_interface)?;
        let block2_class = Self::load_system_class(classpath.as_slice(), "Block2", Some(block_class.clone()), &mut gc_interface)?;
        let block3_class = Self::load_system_class(classpath.as_slice(), "Block3", Some(block_class.clone()), &mut gc_interface)?;

        let boolean_class = Self::load_system_class(classpath.as_slice(), "Boolean", Some(object_class.clone()), &mut gc_interface)?;
        let true_class = Self::load_system_class(classpath.as_slice(), "True", Some(boolean_class.clone()), &mut gc_interface)?;
        let false_class = Self::load_system_class(classpath.as_slice(), "False", Some(boolean_class.clone()), &mut gc_interface)?;

        // initializeSystemClass(objectClass, null, "Object");
        // set_super_class(&object_class, &nil_class, &metaclass_class);
        object_class
            .borrow()
            .class()
            .borrow_mut()
            .set_class(&metaclass_class);
        object_class
            .borrow()
            .class()
            .borrow_mut()
            .set_super_class(&class_class);
        
        // initializeSystemClass(classClass, objectClass, "Class");
        set_super_class(&class_class, &object_class, &metaclass_class);
        // initializeSystemClass(metaclassClass, classClass, "Metaclass");
        set_super_class(&metaclass_class, &class_class, &metaclass_class);
        // initializeSystemClass(nilClass, objectClass, "Nil");
        set_super_class(&nil_class, &object_class, &metaclass_class);
        // initializeSystemClass(arrayClass, objectClass, "Array");
        set_super_class(&array_class, &object_class, &metaclass_class);
        // initializeSystemClass(methodClass, arrayClass, "Method");
        set_super_class(&method_class, &array_class, &metaclass_class);
        // initializeSystemClass(stringClass, objectClass, "String");
        set_super_class(&string_class, &object_class, &metaclass_class);
        // initializeSystemClass(symbolClass, stringClass, "Symbol");
        set_super_class(&symbol_class, &string_class, &metaclass_class);
        // initializeSystemClass(integerClass, objectClass, "Integer");
        set_super_class(&integer_class, &object_class, &metaclass_class);
        // initializeSystemClass(primitiveClass, objectClass, "Primitive");
        set_super_class(&primitive_class, &object_class, &metaclass_class);
        // initializeSystemClass(doubleClass, objectClass, "Double");
        set_super_class(&double_class, &object_class, &metaclass_class);

        set_super_class(&system_class, &object_class, &metaclass_class);

        set_super_class(&block_class, &object_class, &metaclass_class);
        set_super_class(&block1_class, &block_class, &metaclass_class);
        set_super_class(&block2_class, &block_class, &metaclass_class);
        set_super_class(&block3_class, &block_class, &metaclass_class);

        set_super_class(&boolean_class, &object_class, &metaclass_class);
        set_super_class(&true_class, &boolean_class, &metaclass_class);
        set_super_class(&false_class, &boolean_class, &metaclass_class);

        globals.insert("Object".into(), Value::Class(object_class.clone()));
        globals.insert("Class".into(), Value::Class(class_class.clone()));
        globals.insert("Metaclass".into(), Value::Class(metaclass_class.clone()));
        globals.insert("Nil".into(), Value::Class(nil_class.clone()));
        globals.insert("Integer".into(), Value::Class(integer_class.clone()));
        globals.insert("Array".into(), Value::Class(array_class.clone()));
        globals.insert("Method".into(), Value::Class(method_class.clone()));
        globals.insert("Symbol".into(), Value::Class(symbol_class.clone()));
        globals.insert("Primitive".into(), Value::Class(primitive_class.clone()));
        globals.insert("String".into(), Value::Class(string_class.clone()));
        globals.insert("System".into(), Value::Class(system_class.clone()));
        globals.insert("Double".into(), Value::Class(double_class.clone()));
        globals.insert("Boolean".into(), Value::Class(boolean_class.clone()));
        globals.insert("True".into(), Value::Class(true_class.clone()));
        globals.insert("False".into(), Value::Class(false_class.clone()));
        globals.insert("Block".into(), Value::Class(block_class.clone()));
        globals.insert("Block1".into(), Value::Class(block1_class.clone()));
        globals.insert("Block2".into(), Value::Class(block2_class.clone()));
        globals.insert("Block3".into(), Value::Class(block3_class.clone()));

        globals.insert("true".into(), Value::Boolean(true));
        globals.insert("false".into(), Value::Boolean(false));
        globals.insert("nil".into(), Value::NIL);
        globals.insert("system".into(), Value::SYSTEM);

        Ok(Self {
            globals,
            interner,
            classpath,
            current_frame: GCRef::default(),
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
            gc_interface
        })
    }

    /// Load a class from its name into this universe.
    pub fn load_class(&mut self, class_name: impl Into<String>) -> Result<GCRef<Class>, Error> {
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
            let tokens: Vec<_> = som_lexer::Lexer::new(contents.as_str())
                .skip_comments(true)
                .skip_whitespace(true)
                .collect();

            // Parse class definition from the tokens.
            let defn = match som_parser::parse_file(tokens.as_slice()) {
                Some(defn) => defn,
                None => continue,
            };

            if defn.name != class_name {
                return Err(anyhow!(
                    "{}: class name is different from file name.",
                    path.display(),
                ));
            }

            let super_class = if let Some(ref super_class) = defn.super_class {
                self.lookup_global(super_class)
                    .and_then(Value::as_class)
                    .and_then(|cls| Some(cls))
                    .unwrap_or_else(|| self.load_class(super_class).unwrap())
            } else {
                self.core.object_class.clone()
            };

            let class = Class::from_class_def(defn, Some(super_class), &mut self.gc_interface).map_err(Error::msg)?;
            set_super_class(&class, &super_class, &self.core.metaclass_class);

            /*fn has_duplicated_field(class: &SOMRef<Class>) -> Option<(String, (String, String))> {
                let super_class_iterator = std::iter::successors(Some(class.clone()), |class| {
                    class.borrow().super_class()
                });
                let mut map = HashMap::<String, String>::new();
                for class in super_class_iterator {
                    let class_name = class.borrow().name().to_string();
                    for (field, _) in class.borrow().locals.iter() {
                        let field_name = field.clone();
                        match map.entry(field_name.clone()) {
                            Entry::Occupied(entry) => {
                                return Some((field_name, (class_name, entry.get().clone())))
                            }
                            Entry::Vacant(v) => {
                                v.insert(class_name.clone());
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

            self.globals.insert(
                class.borrow().name().to_string(),
                Value::Class(class.clone()),
            );

            return Ok(class);
        }

        Err(anyhow!("could not find the '{}' class", class_name))
    }

    /// Load a system class (with an incomplete hierarchy).
    pub fn load_system_class(
        classpath: &[impl AsRef<Path>],
        class_name: impl Into<String>,
        super_class: Option<GCRef<Class>>,
        gc_interface: &mut GCInterface
    ) -> Result<GCRef<Class>, Error> {
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
            let tokens: Vec<_> = som_lexer::Lexer::new(contents.as_str())
                .skip_comments(true)
                .skip_whitespace(true)
                .collect();

            // Parse class definition from the tokens.
            let defn = match som_parser::parse_file_no_universe(tokens.as_slice()) {
                Some(defn) => defn,
                None => return Err(anyhow!("could not parse the '{}' system class", class_name)),
            };

            if defn.name != class_name {
                return Err(anyhow!(
                    "{}: class name is different from file name.",
                    path.display(),
                ));
            }

            return Class::from_class_def(defn, super_class, gc_interface).map_err(Error::msg);
        }

        Err(anyhow!("could not find the '{}' system class", class_name))
    }

    /// Get the **Object** class.
    pub fn object_class(&self) -> GCRef<Class> {
        self.core.object_class.clone()
    }

    /// Get the **Nil** class.
    pub fn nil_class(&self) -> GCRef<Class> {
        self.core.nil_class.clone()
    }
    /// Get the **System** class.
    pub fn system_class(&self) -> GCRef<Class> {
        self.core.system_class.clone()
    }

    /// Get the **Symbol** class.
    pub fn symbol_class(&self) -> GCRef<Class> {
        self.core.symbol_class.clone()
    }
    /// Get the **String** class.
    pub fn string_class(&self) -> GCRef<Class> {
        self.core.string_class.clone()
    }
    /// Get the **Array** class.
    pub fn array_class(&self) -> GCRef<Class> {
        self.core.array_class.clone()
    }

    /// Get the **Integer** class.
    pub fn integer_class(&self) -> GCRef<Class> {
        self.core.integer_class.clone()
    }
    /// Get the **Double** class.
    pub fn double_class(&self) -> GCRef<Class> {
        self.core.double_class.clone()
    }

    /// Get the **Block** class.
    pub fn block_class(&self) -> GCRef<Class> {
        self.core.block_class.clone()
    }
    /// Get the **Block1** class.
    pub fn block1_class(&self) -> GCRef<Class> {
        self.core.block1_class.clone()
    }
    /// Get the **Block2** class.
    pub fn block2_class(&self) -> GCRef<Class> {
        self.core.block2_class.clone()
    }
    /// Get the **Block3** class.
    pub fn block3_class(&self) -> GCRef<Class> {
        self.core.block3_class.clone()
    }

    /// Get the **True** class.
    pub fn true_class(&self) -> GCRef<Class> {
        self.core.true_class.clone()
    }
    /// Get the **False** class.
    pub fn false_class(&self) -> GCRef<Class> {
        self.core.false_class.clone()
    }

    /// Get the **Metaclass** class.
    pub fn metaclass_class(&self) -> GCRef<Class> {
        self.core.metaclass_class.clone()
    }

    /// Get the **Method** class.
    pub fn method_class(&self) -> GCRef<Class> {
        self.core.method_class.clone()
    }
    /// Get the **Primitive** class.
    pub fn primitive_class(&self) -> GCRef<Class> {
        self.core.primitive_class.clone()
    }
}

impl Universe {
    /// Execute a piece of code within a new stack frame.
    // pub fn with_frame<T>(&mut self, kind: FrameKind, self_value: Value, nbr_locals: usize, func: impl FnOnce(&mut Self) -> T) -> T {
    //     let frame = Rc::new(RefCell::new(Frame::from_kind(kind, nbr_locals, self_value)));
    //     self.frames.push(frame);
    //     let ret = func(self);
    //     self.frames.pop();
    //     ret
    // }

    pub fn with_frame<T>(&mut self, nbr_locals: u8, args: Vec<Value>, func: impl FnOnce(&mut Self) -> T) -> T {
        let frame = Frame::alloc_new_frame(nbr_locals, args, self.current_frame, &mut self.gc_interface);
        self.current_frame = frame;
        let ret = func(self);
        self.current_frame = frame.to_obj().prev_frame;
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
    pub fn lookup_local(&self, idx: u8) -> Value {
        self.current_frame.lookup_local(idx)
    }

    /// Look up a variable we know to have been defined in another scope.
    pub fn lookup_non_local(&self, idx: u8, target_scope: u8) -> Value {
        Frame::nth_frame_back(&self.current_frame, target_scope).lookup_local(idx)
    }

    /// Look up a field.
    pub fn lookup_field(&self, idx: u8) -> Value {
        self.current_frame.lookup_field(idx)
    }

    pub fn lookup_arg(&self, idx: u8, scope: u8) -> Value {
        Frame::nth_frame_back(&self.current_frame, scope).lookup_argument(idx)
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
        self.current_frame.assign_local(idx, value.clone())
    }

    pub fn assign_non_local(&mut self, idx: u8, scope: u8, value: &Value) {
        Frame::nth_frame_back(&self.current_frame, scope).assign_local(idx, value.clone())
    }

    pub fn assign_field(&mut self, idx: u8, value: &Value) {
        // dbg!(&idx);
        self.current_frame.assign_field(idx, value)
    }

    pub fn assign_arg(&mut self, idx: u8, scope: u8, value: &Value) {
        Frame::nth_frame_back(&self.current_frame, scope).assign_arg(idx, value.clone())
    }

    /// Assign a value to a global binding.
    pub fn assign_global(&mut self, name: impl AsRef<str>, value: &Value) -> Option<()> {
        self.globals
            .insert(name.as_ref().to_string(), value.clone())
            .map(|_| ())
    }
}

impl Universe {
    /// Call `escapedBlock:` on the given value, if it is defined.
    pub fn escaped_block(&mut self, value: Value, block: GCRef<Block>) -> Option<Return> {
        let initialize = value.lookup_method(self, "escapedBlock:")?;

        let escaped_block_result = initialize.to_obj().invoke(self, vec![value, Value::Block(block)]);
        Some(escaped_block_result)
    }

    /// Call `doesNotUnderstand:` on the given value, if it is defined.
    pub fn does_not_understand(
        &mut self,
        value: Value,
        symbol: impl AsRef<str>,
        args: Vec<Value>,
    ) -> Option<Return> {
        let initialize = value.lookup_method(self, "doesNotUnderstand:arguments:")?;
        let sym = self.intern_symbol(symbol.as_ref());
        let sym = Value::Symbol(sym);
        let args = Value::Array(GCRef::<Vec<Value>>::alloc(args, &mut self.gc_interface));

       // eprintln!("Couldn't invoke {}; exiting.", symbol.as_ref()); std::process::exit(1);
        
        let dnu_result = initialize.to_obj().invoke(self, vec![value, sym, args]);
        Some(dnu_result)
    }

    /// Call `unknownGlobal:` on the given value, if it is defined.
    pub fn unknown_global(&mut self, value: Value, name: impl AsRef<str>) -> Option<Return> {
        let sym = self.intern_symbol(name.as_ref());
        let method = value.lookup_method(self, "unknownGlobal:")?;

        let unknown_global_result = method.to_obj().invoke(self, vec![value, Value::Symbol(sym)]);
        match unknown_global_result {
            Return::Local(value) | Return::NonLocal(value, _) => Some(Return::Local(value)),
            Return::Exception(err) => Some(Return::Exception(format!(
                "(from 'System>>#unknownGlobal:') {}",
                err,
            ))),
            Return::Restart => Some(Return::Exception(
                "(from 'System>>#unknownGlobal:') incorrectly asked for a restart".to_string(),
            )),
        }
    }

    /// Call `System>>#initialize:` with the given name, if it is defined.
    pub fn initialize(&mut self, args: Vec<Value>) -> Option<Return> {
        let initialize = Value::SYSTEM.lookup_method(self, "initialize:")?;
        let args = Value::Array(GCRef::<Vec<Value>>::alloc(args, &mut self.gc_interface));

        let program_result = initialize.to_obj().invoke(self, vec![Value::SYSTEM, args]);
        Some(program_result)
    }
}

fn set_super_class(
    class: &GCRef<Class>,
    super_class: &GCRef<Class>,
    metaclass_class: &GCRef<Class>,
) {
    class.borrow_mut().set_super_class(super_class);

    class
        .borrow()
        .class()
        .borrow_mut()
        .set_super_class(&super_class.borrow().class());
    class
        .borrow()
        .class()
        .borrow_mut()
        .set_class(metaclass_class);
}
