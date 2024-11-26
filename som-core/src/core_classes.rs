/// The core classes of the SOM interpreter.
///
/// This struct allows to always keep a reference to important classes,
/// even in case of modifications to global bindings by user-defined code.
#[derive(Debug)]
pub struct CoreClasses<ClassPtr> {
    /// The **Object** class.
    pub object_class: ClassPtr,
    /// The **Class** class.
    pub class_class: ClassPtr,
    /// The **Class** class.
    pub metaclass_class: ClassPtr,

    /// The **Nil** class.
    pub nil_class: ClassPtr,
    /// The **Integer** class.
    pub integer_class: ClassPtr,
    /// The **Double** class.
    pub double_class: ClassPtr,
    /// The **Array** class.
    pub array_class: ClassPtr,
    /// The **Method** class.
    pub method_class: ClassPtr,
    /// The **Primitive** class.
    pub primitive_class: ClassPtr,
    /// The **Symbol** class.
    pub symbol_class: ClassPtr,
    /// The **String** class.
    pub string_class: ClassPtr,
    /// The **System** class.
    pub system_class: ClassPtr,

    /// The **Block** class.
    pub block_class: ClassPtr,
    /// The **Block1** class.
    pub block1_class: ClassPtr,
    /// The **Block2** class.
    pub block2_class: ClassPtr,
    /// The **Block3** class.
    pub block3_class: ClassPtr,

    /// The **Boolean** class.
    pub boolean_class: ClassPtr,
    /// The **True** class.
    pub true_class: ClassPtr,
    /// The **False** class.
    pub false_class: ClassPtr,
}

impl<Class> CoreClasses<Class> {
    pub fn iter(&self) -> CoreClassesIter<Class> {
        CoreClassesIter {
            fields: vec![
                &self.object_class,
                &self.class_class,
                &self.metaclass_class,
                &self.nil_class,
                &self.integer_class,
                &self.double_class,
                &self.array_class,
                &self.method_class,
                &self.primitive_class,
                &self.symbol_class,
                &self.string_class,
                &self.system_class,
                &self.block_class,
                &self.block1_class,
                &self.block2_class,
                &self.block3_class,
                &self.boolean_class,
                &self.true_class,
                &self.false_class,
            ]
            .into_iter(),
        }
    }
}

pub struct CoreClassesIter<'a, ClassPtr> {
    fields: std::vec::IntoIter<&'a ClassPtr>,
}

impl<'a, ClassPtr> Iterator for CoreClassesIter<'a, ClassPtr> {
    type Item = &'a ClassPtr;

    fn next(&mut self) -> Option<Self::Item> {
        self.fields.next()
    }
}
