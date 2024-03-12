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

#[derive(Clone, Debug)]
pub struct AstMethodGenCtxt {
    pub all_locals: Vec<String>,
    pub class_fields: Vec<String>,
    pub current_scope: usize,
    pub outer_ctxt: Option<Box<AstMethodGenCtxt>>,
}

impl Default for AstMethodGenCtxt {
    fn default() -> Self {
        AstMethodGenCtxt{
            all_locals: vec![],
            class_fields: vec![],
            current_scope: 0,
            outer_ctxt: None,
        }
    }
}

impl AstMethodGenCtxt {
    pub fn new_ctxt_from_itself(&self) -> AstMethodGenCtxt {
        AstMethodGenCtxt {
            all_locals: vec![],
            class_fields: self.class_fields.clone(),
            current_scope: self.current_scope + 1,
            outer_ctxt: Some(Box::from(self.clone())),
        }
    }

    pub fn get_outer(&self) -> AstMethodGenCtxt {
        let outer = self.outer_ctxt.as_ref().unwrap();
        *outer.clone()
    }

    pub fn add_fields(&self, fields_names: &Vec<String>) -> AstMethodGenCtxt {
        AstMethodGenCtxt {
            all_locals: self.all_locals.clone(),
            class_fields: fields_names.clone(),
            current_scope: self.current_scope,
            outer_ctxt: self.outer_ctxt.clone(),
        }
    }

    pub fn add_locals(&self, new_locals_names: &Vec<String>) -> AstMethodGenCtxt {
        AstMethodGenCtxt {
            all_locals: new_locals_names.clone(),
            class_fields: self.class_fields.clone(),
            current_scope: self.current_scope,
            outer_ctxt: self.outer_ctxt.clone()
        }
    }

    pub fn get_var(&self, name: &String) -> Option<(String, usize)> {
        self.get_var_rec(name, 0)
    }

    pub fn get_var_rec(&self, name: &String, cur_scope: usize) -> Option<(String, usize)> {
        match self.all_locals.iter().find(|local| *local == name) {
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
}


/// Parses the input of an entire file into an AST.
pub fn parse_file(input: &[Token]) -> Option<ClassDef> {
    self::apply(lang::file(), input)
}

/// Applies a parser and returns the output value if the entirety of the input has been parsed successfully.
pub fn apply<'a, A, P>(mut parser: P, input: &'a [Token]) -> Option<A>
    where
        P: Parser<A, &'a [Token], AstMethodGenCtxt>,
{
    match parser.parse(input, AstMethodGenCtxt::default()) {
        Some((output, tail, _)) if tail.is_empty() => Some(output),
        Some(_) | None => None,
    }
}
