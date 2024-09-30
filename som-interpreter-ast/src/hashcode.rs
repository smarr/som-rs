use std::hash::{Hash, Hasher};

use crate::block::Block;
use crate::class::Class;
use crate::instance::Instance;
use crate::method::Method;

// impl Hash for Value {
//     fn hash<H: Hasher>(&self, hasher: &mut H) {
//         match self {
//             Value::Nil => {
//                 hasher.write(b"#nil#");
//             }
//             Value::System => {
//                 hasher.write(b"#system#");
//             }
//             Value::Boolean(value) => {
//                 hasher.write(b"#bool#");
//                 value.hash(hasher);
//             }
//             Value::Integer(value) => {
//                 hasher.write(b"#int#");
//                 value.hash(hasher);
//             }
//             Value::BigInteger(value) => {
//                 hasher.write(b"#bigint#");
//                 value.to_obj().hash(hasher);
//             }
//             Value::Double(value) => {
//                 hasher.write(b"#double#");
//                 let raw_bytes: &[u8] = unsafe {
//                     std::slice::from_raw_parts(
//                         (value as *const f64) as *const u8,
//                         std::mem::size_of::<f64>(),
//                     )
//                 };
//                 hasher.write(raw_bytes);
//             }
//             Value::Symbol(value) => {
//                 hasher.write(b"#sym#");
//                 value.hash(hasher);
//             }
//             Value::String(value) => {
//                 hasher.write(b"#string#");
//                 value.to_obj().hash(hasher);
//             }
//             Value::Array(value) => {
//                 hasher.write(b"#arr#");
//                 for value in value.borrow().iter() {
//                     value.hash(hasher);
//                 }
//             }
//             Value::Block(value) => {
//                 hasher.write(b"#blk#");
//                 value.borrow().hash(hasher);
//             }
//             Value::Class(value) => {
//                 hasher.write(b"#cls#");
//                 value.borrow().hash(hasher);
//             }
//             Value::Instance(value) => {
//                 hasher.write(b"#inst#");
//                 value.borrow().hash(hasher);
//             }
//             Value::Invokable(value) => {
//                 hasher.write(b"#mthd#");
//                 value.to_obj().hash(hasher);
//             },
//         }
//     }
// }

impl Hash for Class {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        self.name.hash(hasher);
        self.fields.hash(hasher)
    }
}

impl Hash for Instance {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        self.class.borrow().hash(hasher);
        self.locals.hash(hasher)
    }
}

impl Hash for Block {
    fn hash<H: Hasher>(&self, _hasher: &mut H) {
        todo!()
    }
}

impl Hash for Method {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        self.holder.to_obj().hash(hasher);
        hasher.write(b">>");
        self.signature.hash(hasher);
    }
}
