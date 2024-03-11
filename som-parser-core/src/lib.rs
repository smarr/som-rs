use std::marker::PhantomData;

/// Generic parser combinators.
pub mod combinators;

/// Defines a parser.
///
/// It is basically a function that takes an input and returns a parsed result along with the rest of input (which can be parsed further).
pub trait Parser<T, I, MGCTXT>: Sized {
    /// Applies the parser on some input.
    ///
    /// It returns the parsed value and the rest of the unparsed input as `Some`, if successful.  
    /// Failing that, it returns `None`.  
    fn parse(&mut self, input: I, mgctxt: MGCTXT) -> Option<(T, I, MGCTXT)>;

    /// Sequences two parsers, one after the other, collecting both results.
    fn and<U, P: Parser<U, I, MGCTXT>>(self, parser: P) -> And<Self, P> {
        And {
            p1: self,
            p2: parser,
        }
    }

    /// Tries to apply the first parser, if it fails, it tries to apply the second parser.
    fn or<P: Parser<T, I, MGCTXT>>(self, parser: P) -> Or<Self, P> {
        Or {
            p1: self,
            p2: parser,
        }
    }

    /// Maps a function over the output value of the parser.
    fn map<F: Fn(T) -> U, U>(self, func: F) -> Map<Self, F, T> {
        Map {
            parser: self,
            func,
            _phantom: PhantomData,
        }
    }

    /// Sequences two parsers, one after the other, but discards the output of the second one.
    fn and_left<P: Parser<U, I, MGCTXT>, U>(self, parser: P) -> AndLeft<Self, P, U> {
        AndLeft {
            p1: self,
            p2: parser,
            _phantom: PhantomData,
        }
    }

    /// Sequences two parsers, one after the other, but discards the output of the first one.
    fn and_right<P: Parser<U, I, MGCTXT>, U>(self, parser: P) -> AndRight<Self, P, T> {
        AndRight {
            p1: self,
            p2: parser,
            _phantom: PhantomData,
        }
    }
}

/// Sequences two parsers, one after the other, collecting both results.
pub struct And<A, B> {
    p1: A,
    p2: B,
}

impl<T1, T2, A, B, I, MGCTXT> Parser<(T1, T2), I, MGCTXT> for And<A, B>
where
    A: Parser<T1, I, MGCTXT>,
    B: Parser<T2, I, MGCTXT>,
{
    fn parse<'a>(&mut self, input: I, mgctxt: MGCTXT) -> Option<((T1, T2), I, MGCTXT)> {
        let (v1, input, mgctxt) = self.p1.parse(input, mgctxt)?;
        let (v2, input, mgctxt) = self.p2.parse(input, mgctxt)?;
        Some(((v1, v2), input, mgctxt))
    }
}

/// Tries to apply the first parser, if it fails, it tries to apply the second parser.
pub struct Or<A, B> {
    p1: A,
    p2: B,
}

impl<T, A, B, I, MGCTXT> Parser<T, I, MGCTXT> for Or<A, B>
where
    I: Clone,
    MGCTXT: Clone,
    A: Parser<T, I, MGCTXT>,
    B: Parser<T, I, MGCTXT>,
{
    fn parse(&mut self, input: I, mgctxt: MGCTXT) -> Option<(T, I, MGCTXT)> {
        // self.p1
        //     .parse(input.clone(), mgctxt)
        //     .or_else(|| self.p2.parse(input, mgctxt))

        let l1 = self.p1.parse(input.clone(), mgctxt.clone());

        if l1.is_some() {
            l1
        } else {
            self.p2.parse(input, mgctxt)
        }
    }
}

/// Maps a function over the output value of the parser.
pub struct Map<P, F, T> {
    parser: P,
    func: F,
    _phantom: PhantomData<T>,
}

impl<P, T, F, U, I, MGCTXT> Parser<U, I, MGCTXT> for Map<P, F, T>
where
    P: Parser<T, I, MGCTXT>,
    F: Fn(T) -> U,
{
    fn parse<'a>(&mut self, input: I, mgctxt: MGCTXT) -> Option<(U, I, MGCTXT)> {
        let (value, input, mgctxt) = self.parser.parse(input, mgctxt)?;
        Some(((self.func)(value), input, mgctxt))
    }
}

/// Sequences two parsers, one after the other, but discards the output of the second one.
pub struct AndLeft<A, B, U> {
    p1: A,
    p2: B,
    _phantom: PhantomData<U>,
}

impl<A, B, T, U, I, MGCTXT> Parser<T, I, MGCTXT> for AndLeft<A, B, U>
where
    A: Parser<T, I, MGCTXT>,
    B: Parser<U, I, MGCTXT>,
{
    fn parse(&mut self, input: I, mgctxt: MGCTXT) -> Option<(T, I, MGCTXT)> {
        let (value, input, mgctxt) = self.p1.parse(input, mgctxt)?;
        let (_, input, mgctxt) = self.p2.parse(input, mgctxt)?;
        Some((value, input, mgctxt))
    }
}

/// Sequences two parsers, one after the other, but discards the output of the first one.
pub struct AndRight<A, B, T> {
    p1: A,
    p2: B,
    _phantom: PhantomData<T>,
}

impl<A, B, T, U, I, MGCTXT> Parser<U, I, MGCTXT> for AndRight<A, B, T>
where
    A: Parser<T, I, MGCTXT>,
    B: Parser<U, I, MGCTXT>,
{
    fn parse(&mut self, input: I, mgctxt: MGCTXT) -> Option<(U, I, MGCTXT)> {
        let (_, input, mgctxt) = self.p1.parse(input, mgctxt)?;
        let (value, input, mgctxt) = self.p2.parse(input, mgctxt)?;
        Some((value, input, mgctxt))
    }
}

/// Because a `Parser` is basically a function of the following signature.
/// ```text
/// (I) -> (T, I)
/// ```
/// We can implement it for any bare `Fn(I) -> (T, I)`.
impl<T, F, I, MGCTXT> Parser<T, I, MGCTXT> for F
where
    F: FnMut(I, MGCTXT) -> Option<(T, I, MGCTXT)>,
{
    fn parse(&mut self, input: I, mgctxt: MGCTXT) -> Option<(T, I, MGCTXT)> {
        (self)(input, mgctxt)
    }
}
