//!
//! This crate serves as the syntactical analyser (parser) for the Simple Object Machine.
//!
//! This particular version of the parser works with the tokens outputted by the lexical analyser, instead of directly reading text.
//!

/// SOM-specific parser combinators.
pub mod lang;

use som_core::ast::ClassDef;
use som_lexer::Token;
use som_parser_core::{Parser};

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum AstGenCtxtType {
    Class,
    Block,
    Method,
}

#[derive(Clone, Debug)]
pub struct AstGenCtxt {
    kind: AstGenCtxtType, // used for debugging
    name: String, // debugging too
    local_names: Vec<String>,
    param_names: Vec<String>,
    class_field_names: Vec<String>,
    current_scope: usize,
    outer_ctxt: Option<Box<AstGenCtxt>>,
}

impl Default for AstGenCtxt {
    fn default() -> Self {
        AstGenCtxt {
            kind: AstGenCtxtType::Class,
            name: "NO NAME".to_string(),
            local_names: vec![],
            param_names: vec![],
            class_field_names: vec![],
            current_scope: 0,
            outer_ctxt: None,
        }
    }
}

impl AstGenCtxt {
    pub fn new_ctxt_from_itself(&self, kind: AstGenCtxtType) -> AstGenCtxt {
        AstGenCtxt {
            kind,
            name: "NO NAME".to_string(),
            local_names: vec![],
            param_names: vec![],
            class_field_names: self.class_field_names.clone(),
            current_scope: self.current_scope + 1,
            outer_ctxt: Some(Box::from(self.clone())),
        }
    }

    pub fn set_name(&self, name: String) -> AstGenCtxt {
        AstGenCtxt {
            kind: self.kind,
            name,
            local_names: self.local_names.clone(),
            param_names: self.param_names.clone(),
            class_field_names: self.class_field_names.clone(),
            current_scope: self.current_scope,
            outer_ctxt: self.outer_ctxt.clone(),
        }
    }

    pub fn get_outer(&self) -> AstGenCtxt {
        let outer = self.outer_ctxt.as_ref().unwrap();
        *outer.clone()
    }

    pub fn add_fields(&self, fields_names: &Vec<String>) -> AstGenCtxt {
        AstGenCtxt {
            kind: self.kind,
            name: self.name.clone(),
            local_names: self.local_names.clone(),
            param_names: self.param_names.clone(),
            class_field_names: fields_names.clone(),
            current_scope: self.current_scope,
            outer_ctxt: self.outer_ctxt.clone(),
        }
    }

    pub fn add_locals(&self, new_locals_names: &Vec<String>) -> AstGenCtxt {
        AstGenCtxt {
            kind: self.kind,
            name: self.name.clone(),
            local_names: new_locals_names.clone(),
            param_names: self.param_names.clone(),
            class_field_names: self.class_field_names.clone(),
            current_scope: self.current_scope,
            outer_ctxt: self.outer_ctxt.clone(),
        }
    }

    pub fn add_params(&self, parameters: &Vec<String>) -> AstGenCtxt {
        assert_ne!(self.kind, AstGenCtxtType::Class); // can't add parameters to a class.
        AstGenCtxt {
            kind: self.kind,
            name: self.name.clone(),
            local_names: self.local_names.clone(),
            param_names: parameters.clone(),
            class_field_names: self.class_field_names.clone(),
            current_scope: self.current_scope,
            outer_ctxt: self.outer_ctxt.clone(),
        }
    }

    pub fn get_var(&self, name: &String) -> Option<(String, usize)> {
        self.get_var_rec(name, 0)
    }

    fn get_var_rec(&self, name: &String, cur_scope: usize) -> Option<(String, usize)> {
        match self.local_names.iter().find(|local| *local == name) {
            Some(a) => Some((a.clone(), cur_scope)),
            None => {
                if self.outer_ctxt.is_none() {
                    None
                } else {
                    self.outer_ctxt.as_ref().unwrap().get_var_rec(name, cur_scope + 1)
                }
            }
        }
    }

    pub fn get_param(&self, name: &String) -> Option<(String, usize)> {
        self.get_param_rec(name, 0)
    }

    fn get_param_rec(&self, name: &String, cur_scope: usize) -> Option<(String, usize)> {
        match self.param_names.iter().find(|local| *local == name) {
            Some(a) => Some((a.clone(), cur_scope)),
            None => {
                if self.outer_ctxt.is_none() {
                    None
                } else {
                    self.outer_ctxt.as_ref().unwrap().get_param_rec(name, cur_scope + 1)
                }
            }
        }
    }
}


/// Parses the input of an entire file into an AST.
pub fn parse_file(input: &[Token]) -> Option<ClassDef> {
    self::apply(lang::file(), input)
}

/// Applies a parser and returns the output value if the entirety of the input has been parsed successfully.
pub fn apply<'a, A, P>(mut parser: P, input: &'a [Token]) -> Option<A>
    where
        P: Parser<A, &'a [Token], AstGenCtxt>,
{
    match parser.parse(input, AstGenCtxt::default()) {
        Some((output, tail, _)) if tail.is_empty() => Some(output),
        Some(_) | None => None,
    }
}
