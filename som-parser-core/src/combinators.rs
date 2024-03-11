use crate::{Parser};

/// Represents a value of either type A (Left) or type B (Right).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Either<A, B> {
    Left(A),
    Right(B),
}

/// Transforms a parser into a non-consuming one, allowing to parse ahead without consuming anything.
pub fn peek<A, I: Clone, MGCTXT>(mut parser: impl Parser<A, I, MGCTXT>) -> impl Parser<A, I, MGCTXT> {
    move |input: I, mgctxt: MGCTXT| {
        let (value, _, mgctxt) = parser.parse(input.clone(), mgctxt)?;
        Some((value, input, mgctxt))
    }
}

// /// Runs the given parser, fails if it succeeded, and succeeds otherwise.
// pub fn not<A, I: Clone, MGCTXT>(mut parser: impl Parser<A, I, MGCTXT>) -> impl Parser<(), I, MGCTXT> {
//     move |input: I, mgctxt: &mut MGCTXT| match parser.parse(input.clone(), mgctxt) {
//         Some(_) => None,
//         None => Some(((), input)),
//     }
// }
//
// /// Sequences two parsers, one after the other, collecting both results.
// pub fn sequence<A, B, I, MGCTXT>(
//     mut fst: impl Parser<A, I, MGCTXT>,
//     mut snd: impl Parser<B, I, MGCTXT>,
// ) -> impl Parser<(A, B), I, MGCTXT> {
//     // equivalent to: `fst.and(snd)`
//     move |input: I, mgctxt: &mut MGCTXT| {
//         let (a, input) = fst.parse(input, mgctxt)?;
//         let (b, input) = snd.parse(input, mgctxt)?;
//         Some(((a, b), input))
//     }
// }
//
// /// Tries to apply the first parser, if it fails, it tries to apply the second parser.
// pub fn alternative<A, I: Clone, MGCTXT>(
//     mut fst: impl Parser<A, I, MGCTXT>,
//     mut snd: impl Parser<A, I, MGCTXT>,
// ) -> impl Parser<A, I, MGCTXT> {
//     move |input: I, mgctxt: &mut MGCTXT| fst.parse(input.clone(), mgctxt).or_else(|| snd.parse(input, mgctxt))
// }
//
// /// Same as `either`, but allows for different output types for the parsers.
// pub fn either<A, B, I: Clone, MGCTXT>(
//     mut fst: impl Parser<A, I, MGCTXT>,
//     mut snd: impl Parser<B, I, MGCTXT>,
// ) -> impl Parser<Either<A, B>, I, MGCTXT> {
//     move |input: I, mgctxt: &mut MGCTXT| {
//         if let Some((a, input)) = fst.parse(input.clone(), mgctxt) {
//             Some((Either::Left(a), input))
//         } else if let Some((b, input)) = snd.parse(input, mgctxt) {
//             Some((Either::Right(b), input))
//         } else {
//             None
//         }
//     }
// }
//
// /// Tries to apply a parser, or fallback to a constant value (making it an always-succeeding parser).
// pub fn fallback<A: Clone, I: Clone, MGCTXT>(def: A, mut parser: impl Parser<A, I, MGCTXT>) -> impl Parser<A, I, MGCTXT> {
//     move |input: I, mgctxt: &mut MGCTXT| {
//         parser
//             .parse(input.clone(), mgctxt)
//             .or_else(|| Some((def.clone(), input)))
//     }
// }

/// Tries to apply a parser, or fallback to its default value (making it an always-succeeding parser).
pub fn default<A: Default, I: Clone, MGCTXT: Clone>(parser: impl Parser<A, I, MGCTXT>) -> impl Parser<A, I, MGCTXT> {
    optional(parser).map(Option::unwrap_or_default)
}

// /// Tries every parser in a slice, from left to right, and returns the output of the first succeeding one.
// pub fn any<'a, A, I: Clone, MGCTXT>(parsers: &'a mut [impl Parser<A, I, MGCTXT>]) -> impl Parser<A, I, MGCTXT> + 'a {
//     move |input: I, mgctxt: &mut MGCTXT| {
//         parsers
//             .iter_mut()
//             .find_map(|parser| parser.parse(input.clone(), mgctxt))
//     }
// }
//
// /// Applies every parser in a slice, from left to right, and returns the output from all of them.
// /// If one parser fails, the whole sequence is considered failed.
// pub fn all<'a, A, I, MGCTXT>(parsers: &'a mut [impl Parser<A, I, MGCTXT>]) -> impl Parser<Vec<A>, I, MGCTXT> + 'a {
//     move |input: I, mgctxt| {
//         let output = Vec::<A>::with_capacity(parsers.len());
//         parsers
//             .iter_mut()
//             .try_fold((output, input), |(mut output, input), parser| {
//                 let (value, input) = parser.parse(input, mgctxt)?;
//                 output.push(value);
//                 Some((output, input))
//             })
//     }
// }
//
/// Tries to apply a parser, but fails gracefully (with an `Option` output).
pub fn optional<A, I: Clone, MGCTXT: Clone>(mut parser: impl Parser<A, I, MGCTXT>) -> impl Parser<Option<A>, I, MGCTXT> {
    move |input: I, mgctxt: MGCTXT| {
        if let Some((value, input, mgctxt)) = parser.parse(input.clone(), mgctxt.clone()) {
            Some((Some(value), input, mgctxt))
        } else {
            Some((None, input, mgctxt.clone()))
        }
    }
}

/// Applies a parser zero or more times.
pub fn many<A, I: Clone, MGCTXT: Clone>(mut parser: impl Parser<A, I, MGCTXT>) -> impl Parser<Vec<A>, I, MGCTXT> {
    move |mut input: I, mgctxt: MGCTXT| {
        let mut output = Vec::<A>::new();
        let mut mgctxt2 = mgctxt;
        loop {
            let Some((value, next, modified_ctxt)) = parser.parse(input.clone(), mgctxt2.clone()) else { break };
            mgctxt2 = modified_ctxt;
            input = next;
            output.push(value);
        }
        Some((output, input, mgctxt2))
    }
}

/// Applies a parser one or more times.
pub fn some<A, I: Clone, MGCTXT: Clone>(mut parser: impl Parser<A, I, MGCTXT>) -> impl Parser<Vec<A>, I, MGCTXT> {
    move |input: I, mgctxt| {
        let (value, mut input, mgctxt) = parser.parse(input, mgctxt)?;
        let mut output = vec![value];
        let mut mgctxt2 = mgctxt;

        loop {
            let Some((value, next, new_ctxt)) = parser.parse(input.clone(), mgctxt2.clone()) else { break };
            mgctxt2 = new_ctxt;
            input = next;
            output.push(value);
        }

        Some((output, input, mgctxt2))
    }
}

/// Parses something that is enclosed between two other things.
pub fn between<A, B, C, I, MGCTXT>(
    mut before: impl Parser<A, I, MGCTXT>,
    mut within: impl Parser<B, I, MGCTXT>,
    mut after: impl Parser<C, I, MGCTXT>,
) -> impl Parser<B, I, MGCTXT> {
    move |input: I, mgctxt| {
        let (_, input, mgctxt) = before.parse(input, mgctxt)?;
        let (value, input, mgctxt) = within.parse(input, mgctxt)?;
        let (_, input, mgctxt) = after.parse(input, mgctxt)?;
        Some((value, input, mgctxt))
    }
}

/// Parses zero or more things, separated by an arbitrary delimiter.
pub fn sep_by<A, B, I: Clone, MGCTXT: Clone>(
    mut delim: impl Parser<A, I, MGCTXT>,
    mut within: impl Parser<B, I, MGCTXT>,
) -> impl Parser<Vec<B>, I, MGCTXT> {
    move |input: I, mgctxt: MGCTXT| {
        let mut output = Vec::<B>::new();
        let l1 = within.parse(input.clone(), mgctxt.clone());
        if l1.is_some() {
            let (value, mut input, mut mgctxt2) = l1.unwrap();
            output.push(value);

            loop {
                let l2 = delim
                    .parse(input.clone(), mgctxt2.clone())
                    .and_then(|(_, input, mgctxt3)| within.parse(input, mgctxt3));

                if l2.is_none() {
                    break;
                } else {
                    let (value, next, new_mg) = l2.unwrap();
                    input = next;
                    output.push(value);
                    mgctxt2 = new_mg;
                }
            }

            // while let Some((value, next, mgctxt)) = delim
            //     .parse(input.clone(), mgctxt.clone())
            //     .and_then(|(_, input, mg)| within.parse(input, mg))
            // {
            //     input = next;
            //     output.push(value);
            // }
            Some((output, input, mgctxt2))
        } else {
            Some((output, input, mgctxt.clone()))
        }
    }
}

// /// Parses one or more things, separated by an arbitrary delimiter.
// pub fn sep_by1<A, B, I: Clone, MGCTXT>(
//     mut delim: impl Parser<A, I, MGCTXT>,
//     mut within: impl Parser<B, I, MGCTXT>,
// ) -> impl Parser<Vec<B>, I, MGCTXT> {
//     move |input: I, mgctxt| {
//         let mut output = Vec::<B>::new();
//         let (value, mut input) = within.parse(input, mgctxt)?;
//         output.push(value);
//         while let Some((value, next)) = delim
//             .parse(input.clone(), mgctxt)
//             .and_then(|(_, input)| within.parse(input, mgctxt))
//         {
//             input = next;
//             output.push(value);
//         }
//         Some((output, input))
//     }
// }

/// Transforms the output value of a parser.
pub fn map<A, B, I, MGCTXT>(mut parser: impl Parser<A, I, MGCTXT>, func: impl Fn(A) -> B) -> impl Parser<B, I, MGCTXT> {
    move |input: I, mgctxt: MGCTXT| {
        let (value, input, mgctxt) = parser.parse(input, mgctxt)?;
        Some((func(value), input, mgctxt))
    }
}
