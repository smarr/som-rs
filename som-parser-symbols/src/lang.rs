use som_core::ast::*;
use som_lexer::Token;
use som_parser_core::combinators::*;
use som_parser_core::{Parser};
use crate::{AstGenCtxt, AstGenCtxtType};

macro_rules! opaque {
    ($expr:expr) => {{
        move |input: &'a [Token], genctxt| $expr.parse(input, genctxt)
    }};
}

/// A parser that expects to be nothing left in its input.
pub fn eof<'a>() -> impl Parser<(), &'a [Token], AstGenCtxt> {
    move |input: &'a [Token], genctxt: AstGenCtxt| {
        if input.is_empty() {
            Some(((), input, genctxt))
        } else {
            None
        }
    }
}

pub fn exact<'a>(ch: Token) -> impl Parser<(), &'a [Token], AstGenCtxt> {
    move |input: &'a [Token], genctxt: AstGenCtxt| {
        let (head, tail) = input.split_first()?;
        if *head == ch {
            Some(((), tail, genctxt))
        } else {
            None
        }
    }
}

pub fn exact_ident<'a, 'b: 'a>(string: &'b str) -> impl Parser<(), &'a [Token], AstGenCtxt> {
    move |input: &'a [Token], genctxt| {
        let (head, tail) = input.split_first()?;
        match head {
            Token::Identifier(ref ident) if ident.as_str() == string => Some(((), tail, genctxt)),
            _ => None,
        }
    }
}

pub fn big_integer<'a>() -> impl Parser<String, &'a [Token], AstGenCtxt> {
    move |input: &'a [Token], genctxt| {
        let (sign, input, genctxt) = optional(exact(Token::Minus)).parse(input, genctxt)?;
        let sign = if sign.is_some() { "-" } else { "" };

        let (head, tail) = input.split_first()?;
        match head {
            Token::LitBigInteger(value) => Some((format!("{}{}", sign, value), tail, genctxt)),
            _ => None,
        }
    }
}

pub fn integer<'a>() -> impl Parser<i64, &'a [Token], AstGenCtxt> {
    move |input: &'a [Token], genctxt| {
        let (sign, input, genctxt) = optional(exact(Token::Minus)).parse(input, genctxt)?;
        let sign = if sign.is_some() { -1 } else { 1 };

        let (head, tail) = input.split_first()?;
        match head {
            Token::LitInteger(value) => Some((*value * sign, tail, genctxt)),
            _ => None,
        }
    }
}

pub fn double<'a>() -> impl Parser<f64, &'a [Token], AstGenCtxt> {
    move |input: &'a [Token], genctxt| {
        let (sign, input, genctxt) = optional(exact(Token::Minus)).parse(input, genctxt)?;
        let sign = if sign.is_some() { -1.0 } else { 1.0 };

        let (head, tail) = input.split_first()?;
        match head {
            Token::LitDouble(value) => Some((*value * sign, tail, genctxt)),
            _ => None,
        }
    }
}

pub fn single_operator<'a>() -> impl Parser<&'static str, &'a [Token], AstGenCtxt> {
    move |input: &'a [Token], genctxt| {
        let (head, tail) = input.split_first()?;
        match head {
            Token::Not => Some(("~", tail, genctxt)),
            Token::And => Some(("&", tail, genctxt)),
            Token::Or => Some(("|", tail, genctxt)),
            Token::Star => Some(("*", tail, genctxt)),
            Token::Div => Some(("/", tail, genctxt)),
            Token::Mod => Some(("\\", tail, genctxt)),
            Token::Plus => Some(("+", tail, genctxt)),
            Token::Equal => Some(("=", tail, genctxt)),
            Token::More => Some((">", tail, genctxt)),
            Token::Less => Some(("<", tail, genctxt)),
            Token::Comma => Some((",", tail, genctxt)),
            Token::At => Some(("@", tail, genctxt)),
            Token::Per => Some(("%", tail, genctxt)),
            Token::Minus => Some(("-", tail, genctxt)),
            _ => None,
        }
    }
}

pub fn operator_sequence<'a>() -> impl Parser<String, &'a [Token], AstGenCtxt> {
    move |input: &'a [Token], genctxt| {
        let (head, tail) = input.split_first()?;
        match head {
            Token::OperatorSequence(seq) => Some((seq.clone(), tail, genctxt)),
            _ => None,
        }
    }
}

pub fn operator<'a>() -> impl Parser<String, &'a [Token], AstGenCtxt> {
    single_operator().map(String::from).or(operator_sequence())
}

pub fn identifier<'a>() -> impl Parser<String, &'a [Token], AstGenCtxt> {
    move |input: &'a [Token], genctxt: AstGenCtxt| {
        let (head, tail) = input.split_first()?;
        match head {
            Token::Identifier(value) => {
                Some((value.clone(), tail, genctxt))
            }
            _ => None,
        }
    }
}

pub fn string<'a>() -> impl Parser<String, &'a [Token], AstGenCtxt> {
    move |input: &'a [Token], genctxt| {
        let (head, tail) = input.split_first()?;
        match head {
            Token::LitString(value) => Some((value.clone(), tail, genctxt)),
            _ => None,
        }
    }
}

pub fn symbol<'a>() -> impl Parser<String, &'a [Token], AstGenCtxt> {
    move |input: &'a [Token], genctxt| {
        let (head, tail) = input.split_first()?;
        match head {
            Token::LitSymbol(value) => Some((value.clone(), tail, genctxt)),
            _ => None,
        }
    }
}

pub fn array<'a>() -> impl Parser<Vec<Literal>, &'a [Token], AstGenCtxt> {
    move |input: &'a [Token], genctxt| {
        between(
            exact(Token::NewArray),
            many(literal()),
            exact(Token::EndTerm),
        )
            .parse(input, genctxt)
    }
}

pub fn literal<'a>() -> impl Parser<Literal, &'a [Token], AstGenCtxt> {
    (double().map(Literal::Double))
        .or(integer().map(Literal::Integer))
        .or(big_integer().map(Literal::BigInteger))
        .or(string().map(Literal::String))
        .or(symbol().map(Literal::Symbol))
        .or(array().map(Literal::Array))
}

pub fn keyword<'a>() -> impl Parser<String, &'a [Token], AstGenCtxt> {
    move |input: &'a [Token], genctxt| {
        let (head, tail) = input.split_first()?;
        match head {
            Token::Keyword(value) => Some((value.clone(), tail, genctxt)),
            _ => None,
        }
    }
}

pub fn unary_send<'a>() -> impl Parser<Expression, &'a [Token], AstGenCtxt> {
    opaque!(primary())
        .and(many(identifier()))
        .map(|(receiver, signatures)| {
            signatures
                .into_iter()
                .fold(receiver, |receiver, signature| {
                    Expression::Message(Message {
                        receiver: Box::new(receiver),
                        signature,
                        values: Vec::new(),
                    })
                })
        })
}

pub fn binary_send<'a>() -> impl Parser<Expression, &'a [Token], AstGenCtxt> {
    unary_send()
        .and(many(operator().and(unary_send().map(Box::new))))
        .map(|(lhs, operands)| {
            operands.into_iter().fold(lhs, |lhs, (op, rhs)| {
                Expression::BinaryOp(BinaryOp {
                    lhs: Box::new(lhs),
                    op,
                    rhs,
                })
            })
        })
}

pub fn positional_send<'a>() -> impl Parser<Expression, &'a [Token], AstGenCtxt> {
    binary_send()
        .and(many(keyword().and(binary_send())))
        .map(|(receiver, pairs)| {
            if pairs.is_empty() {
                receiver
            } else {
                let (signature, values) = pairs.into_iter().unzip();

                Expression::Message(Message {
                    receiver: Box::new(receiver),
                    signature,
                    values,
                })
            }
        })
}

pub fn body<'a>() -> impl Parser<Body, &'a [Token], AstGenCtxt> {
    sep_by(exact(Token::Period), exit().or(statement()))
        .and(optional(exact(Token::Period)))
        .map(|(exprs, stopped)| Body {
            exprs,
            full_stopped: stopped.is_some(),
        })
}

pub fn locals<'a>() -> impl Parser<Vec<String>, &'a [Token], AstGenCtxt> {
    move |input: &'a [Token], genctxt| {
        let (new_locals_names, input, mut genctxt) = between(exact(Token::Or), many(identifier()), exact(Token::Or)).parse(input, genctxt)?;
        genctxt = genctxt.add_locals(&new_locals_names);
        Some((new_locals_names, input, genctxt))
    }
}

pub fn class_locals<'a>() -> impl Parser<Vec<String>, &'a [Token], AstGenCtxt> {
    move |input: &'a [Token], genctxt| {
        let (new_locals_names, input, mut genctxt) = between(exact(Token::Or), many(identifier()), exact(Token::Or)).parse(input, genctxt)?;
        genctxt = genctxt.add_fields(&new_locals_names);
        Some((new_locals_names, input, genctxt))
    }
}

pub fn parameter<'a>() -> impl Parser<String, &'a [Token], AstGenCtxt> {
    exact(Token::Colon).and_right(identifier())
}

pub fn parameters<'a>() -> impl Parser<Vec<String>, &'a [Token], AstGenCtxt> {
    move |input: &'a [Token], genctxt: AstGenCtxt| {
        let (param_names, input, mut genctxt) = some(parameter()).and_left(exact(Token::Or)).parse(input, genctxt)?;
        genctxt = genctxt.add_params(&param_names);
        Some((param_names, input, genctxt))
    }
}

pub fn block<'a>() -> impl Parser<Expression, &'a [Token], AstGenCtxt> {
    // between(
    //     exact(Token::NewBlock),
    //     default(parameters()).and(default(locals())).and(body()),
    //     exact(Token::EndBlock),
    // )
    //     .map(|((parameters, locals), body)| {
    //         Expression::Block(Block {
    //             parameters,
    //             locals,
    //             body,
    //         })
    //     })


    move |input: &'a [Token], genctxt| {
        let (_, input, mut genctxt) = exact(Token::NewBlock).parse(input, genctxt)?;
        genctxt = genctxt.new_ctxt_from_itself(AstGenCtxtType::Block);
        genctxt = genctxt.set_name("anonymous block".to_string());
        let (((parameters, locals), body), input, genctxt) = default(parameters())
            .and(default(locals()))
            .and(body())
            .parse(input, genctxt).unwrap();
        // we unwrap here at the risk of panicking since if it panics, well, we would want to adjust the scope. but atm we just crash instead (right?)

        let (_, input, mut genctxt) = exact(Token::EndBlock).parse(input, genctxt)?;
        genctxt = genctxt.get_outer();

        Some((Expression::Block(Block {
            parameters,
            locals,
            body,
        }), input, genctxt))
    }
}

pub fn term<'a>() -> impl Parser<Expression, &'a [Token], AstGenCtxt> {
    between(
        exact(Token::NewTerm),
        assignment().or(expression()),
        exact(Token::EndTerm),
    )
}

pub fn exit<'a>() -> impl Parser<Expression, &'a [Token], AstGenCtxt> {
    exact(Token::Exit)
        .and_right(statement())
        .map(|expr| Expression::Exit(Box::new(expr)))
}

pub fn expression<'a>() -> impl Parser<Expression, &'a [Token], AstGenCtxt> {
    positional_send()
}

pub fn primary<'a>() -> impl Parser<Expression, &'a [Token], AstGenCtxt> {
    move |input: &'a [Token], genctxt: AstGenCtxt| {
        let name_opt = identifier().parse(input, genctxt.clone());

        if name_opt.is_none() {
            return term()
                .or(block())
                .or(literal().map(Expression::Literal)).parse(input, genctxt);
        }

        let (name, input, genctxt) = name_opt.unwrap();

        genctxt.get_var(&name).and_then(|(_, scope)|
            {
                match scope {
                    0 => Some((Expression::LocalVarRead(name.clone()), input, genctxt.clone())),
                    _ => Some((Expression::NonLocalVarRead(name.clone(), scope), input, genctxt.clone()))
                }
            })
            .or(genctxt.get_param(&name).and_then(|_| Some((Expression::ArgRead(name.clone()), input, genctxt.clone()))))
            .or((name.as_str() == "self").then_some((Expression::ArgRead(name.clone()), input, genctxt.clone()))) // bit lame i thiiink?
            .or(genctxt.class_field_names.iter().find(|v| **v == name).and_then(|_| Some((Expression::FieldRead(name.clone()), input, genctxt.clone()))))
            .or(Some((Expression::GlobalRead(name.clone()), input, genctxt.clone())))
    }
}

pub fn assignment<'a>() -> impl Parser<Expression, &'a [Token], AstGenCtxt> {
    // identifier()
    //     .and_left(exact(Token::Assign))
    //     .and(opaque!(statement()))
    //     .map(|(name, expr)| Expression::GlobalWrite(name, Box::new(expr)))
    move |input: &'a [Token], genctxt: AstGenCtxt| {
        let (name, input, genctxt) = identifier().and_left(exact(Token::Assign)).parse(input, genctxt).unwrap();
        let (expr, input, genctxt) = opaque!(statement()).parse(input, genctxt).unwrap();

        // it's stupid we have to clone expr in this bit. can this be avoided?
        genctxt.get_var(&name).and_then(|(_, scope)|
            {
                match scope {
                    0 => Some((Expression::LocalVarWrite(name.clone(), Box::new(expr.clone())), input, genctxt.clone())),
                    _ => Some((Expression::NonLocalVarWrite(name.clone(), scope, Box::new(expr.clone())), input, genctxt.clone()))
                }
            })
            .or(genctxt.get_param(&name).and_then(|_| Some((Expression::ArgWrite(name.clone(), Box::new(expr.clone())), input, genctxt.clone()))))
            .or((name.as_str() == "self").then_some((Expression::ArgWrite(name.clone(), Box::new(expr.clone())), input, genctxt.clone()))) // bit lame i thiiink?
            .or(genctxt.class_field_names.iter().find(|v| **v == name).and_then(|_| Some((Expression::FieldWrite(name.clone(), Box::new(expr.clone())), input, genctxt.clone()))))
            .or(Some((Expression::GlobalWrite(name.clone(), Box::new(expr.clone())), input, genctxt.clone())))
    }
}

pub fn statement<'a>() -> impl Parser<Expression, &'a [Token], AstGenCtxt> {
    assignment().or(expression())
}

pub fn primitive<'a>() -> impl Parser<MethodBody, &'a [Token], AstGenCtxt> {
    exact(Token::Primitive).map(|_| MethodBody::Primitive)
}

pub fn method_body<'a>() -> impl Parser<MethodBody, &'a [Token], AstGenCtxt> {
    between(
        exact(Token::NewTerm),
        default(locals()).and(body()),
        exact(Token::EndTerm),
    )
    .map(|(locals, body)| MethodBody::Body { locals, body })
}

pub fn unary_method_def<'a>() -> impl Parser<MethodDef, &'a [Token], AstGenCtxt> {
    identifier()
        .and_left(exact(Token::Equal))
        .and(primitive().or(method_body()))
        .map(|(signature, body)| MethodDef {
            kind: MethodKind::Unary,
            signature,
            body,
        })
}

pub fn positional_method_def<'a>() -> impl Parser<MethodDef, &'a [Token], AstGenCtxt> {
    move |input: &'a [Token], genctxt: AstGenCtxt| {
        let (pairs, input, mut genctxt) = some(keyword().and(identifier())).and_left(exact(Token::Equal)).parse(input, genctxt)?;
        let (signature, parameters): (String, Vec<String>) = pairs.into_iter().unzip();

        genctxt = genctxt.set_name(signature.clone());
        genctxt = genctxt.add_params(&parameters);

        let (body, input, genctxt) = primitive().or(method_body()).parse(input, genctxt)?;

        let method_def = MethodDef {
            kind: MethodKind::Positional { parameters: parameters.clone() },
            signature,
            body,
        };

        Some((method_def, input, genctxt))
    }
}

pub fn operator_method_def<'a>() -> impl Parser<MethodDef, &'a [Token], AstGenCtxt> {
    move |input: &'a [Token], genctxt: AstGenCtxt| {
        let ((op, rhs), input, mut genctxt) = operator().and(identifier()).and_left(exact(Token::Equal)).parse(input, genctxt)?;

        genctxt = genctxt.add_params(&vec![rhs.clone()]);

        primitive().or(method_body())
            .map(|body| MethodDef {
                kind: MethodKind::Operator { rhs: rhs.clone() },
                signature: op.clone(),
                body,
            }).parse(input, genctxt)
    }
}

pub fn method_def<'a>() -> impl Parser<MethodDef, &'a [Token], AstGenCtxt> {
    move |input: &'a [Token], genctxt: AstGenCtxt| {
        let genctxt = genctxt.new_ctxt_from_itself(AstGenCtxtType::Method);
        let method_def_opt = unary_method_def()
            .or(positional_method_def())
            .or(operator_method_def()).parse(input, genctxt.clone());

        if method_def_opt.is_some() {
            let (method_def, input, mut genctxt) = method_def_opt.unwrap();
            genctxt = genctxt.get_outer();
            Some((method_def, input, genctxt))
        } else {
            None
        }

    }
}

pub fn class_def<'a>() -> impl Parser<ClassDef, &'a [Token], AstGenCtxt> {
    move |input: &'a [Token], genctxt: AstGenCtxt| {
        let (name, input, mut genctxt) = identifier().and_left(exact(Token::Equal)).parse(input, genctxt).unwrap();

        genctxt = genctxt.set_name(name.clone());

        optional(identifier())
            .and(between(
                exact(Token::NewTerm),
                default(class_locals()).and(many(method_def())).and(default(
                    exact(Token::Separator).and_right(default(class_locals()).and(many(method_def()))),
                )),
                exact(Token::EndTerm),
            ))
            .map(|(super_class, (instance_defns, static_defns))| {
                let (instance_locals, instance_methods) = instance_defns;
                let (static_locals, static_methods) = static_defns;

                ClassDef {
                    name: name.clone(),
                    super_class,
                    instance_locals,
                    instance_methods,
                    static_locals,
                    static_methods,
                }
            }).parse(input, genctxt)
    }
}

pub fn file<'a>() -> impl Parser<ClassDef, &'a [Token], AstGenCtxt> {
    class_def().and_left(eof())
}
