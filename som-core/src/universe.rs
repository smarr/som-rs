use std::path::PathBuf;
use anyhow::Error;

pub trait Universe<ClassType> {
    fn with_classpath(classpath: Vec<PathBuf>) -> Result<Self, Error>
        where
            Self: Sized;

    fn load_class(&mut self, class_name: impl Into<String>) -> Result<ClassType, Error>;

    fn nil_class(&self) -> ClassType;
    fn system_class(&self) -> ClassType;
    fn object_class(&self) -> ClassType;
    fn symbol_class(&self) -> ClassType;
    fn string_class(&self) -> ClassType;
    fn array_class(&self) -> ClassType;
    fn integer_class(&self) -> ClassType;
    fn double_class(&self) -> ClassType;
    fn block_class(&self) -> ClassType;
    fn block1_class(&self) -> ClassType;
    fn block2_class(&self) -> ClassType;
    fn block3_class(&self) -> ClassType;
    fn true_class(&self) -> ClassType;
    fn false_class(&self) -> ClassType;
    fn metaclass_class(&self) -> ClassType;
    fn method_class(&self) -> ClassType;
    fn primitive_class(&self) -> ClassType;
}