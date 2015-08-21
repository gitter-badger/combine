use std::fmt;
use std::error::Error as StdError;
use std::any::Any;

///Struct which represents a position in a source file
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct SourcePosition {
    ///Current line of the input
    pub line: i32,
    ///Current column of the input
    pub column: i32
}

///Struct which represents a position in a byte stream
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct BytePosition {
    ///Current position
    pub position: usize
}

impl fmt::Display for BytePosition {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "position: {}", self.position)
    }
}

///Enum holding error information
///As there is implementations of `From` for `T: Positioner`, `String` and `&'static str` the
///constructor need not be used directly as calling `msg.into()` should turn a message into the
///correct `Info` variant
#[derive(Clone, Debug)]
pub enum Info<T, R> {
    Token(T),
    Range(R),
    Owned(String),
    Borrowed(&'static str)
}

impl <T: PartialEq, R: PartialEq> PartialEq for Info<T, R> {
    fn eq(&self, other: &Info<T, R>) -> bool {
        match (self, other) {
            (&Info::Token(ref l), &Info::Token(ref r)) => l == r,
            (&Info::Range(ref l), &Info::Range(ref r)) => l == r,
            (&Info::Owned(ref l), &Info::Owned(ref r)) => l == r,
            (&Info::Borrowed(ref l), &Info::Owned(ref r)) => l == r,
            (&Info::Owned(ref l), &Info::Borrowed(ref r)) => l == r,
            (&Info::Borrowed(ref l), &Info::Borrowed(ref r)) => l == r,
            _ => false
        }
    }
}
impl <T: fmt::Display, R: fmt::Display> fmt::Display for Info<T, R> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Info::Token(ref c) => write!(f, "{}", c),
            Info::Range(ref c) => write!(f, "{}", c),
            Info::Owned(ref s) => write!(f, "{}", s),
            Info::Borrowed(s) => write!(f, "{}", s),
        }
    }
}

impl <R> From<char> for Info<char, R> {
    fn from(s: char) -> Info<char, R> {
        Info::Token(s)
    }
}
impl <T, R> From<String> for Info<T, R> {
    fn from(s: String) -> Info<T, R> {
        Info::Owned(s)
    }
}

impl <T, R> From<&'static str> for Info<T, R> {
    fn from(s: &'static str) -> Info<T, R> {
        Info::Borrowed(s)
    }
}

///Enum used to store information about an error that has occured
#[derive(Debug)]
pub enum Error<T, R> {
    ///Error indicating an unexpected token has been encountered in the stream
    Unexpected(Info<T, R>),
    ///Error indicating that the parser expected something else
    Expected(Info<T, R>),
    ///Generic message
    Message(Info<T, R>),
    ///Variant for containing other types of errors
    Other(Box<StdError+Send>)
}

impl <T: PartialEq, R: PartialEq> PartialEq for Error<T, R> {
    fn eq(&self, other: &Error<T, R>) -> bool {
        match (self, other) {
            (&Error::Unexpected(ref l), &Error::Unexpected(ref r)) => l == r,
            (&Error::Expected(ref l), &Error::Expected(ref r)) => l == r,
            (&Error::Message(ref l), &Error::Message(ref r)) => l == r,
            _ => false
        }
    }
}

impl <E, T, R> From<E> for Error<T, R> where E: StdError + 'static + Send {
    fn from(e: E) -> Error<T, R> {
        Error::Other(Box::new(e))
    }
}

impl <T, R> Error<T, R> {
    pub fn end_of_input() -> Error<T, R> {
        Error::Message("End of input".into())
    }
}

///Enum used to indicate if a parser consumed any items of the stream it was given as an input
#[derive(Clone, PartialEq, Debug, Copy)]
pub enum Consumed<T> {
    ///Constructor indicating that the parser has consumed elements
    Consumed(T),
    ///Constructor indicating that the parser did not consume any elements
    Empty(T)
}

impl <T> Consumed<T> {

    ///Returns true if `self` is empty
    pub fn is_empty(&self) -> bool {
        match *self {
            Consumed::Empty(_) => true,
            Consumed::Consumed(_) => false
        }
    }

    ///Extracts the contained value
    pub fn into_inner(self) -> T {
        match self {
            Consumed::Empty(x) => x,
            Consumed::Consumed(x) => x
        }
    }

    ///Converts `self` into the Consumed state
    pub fn as_consumed(self) -> Consumed<T> {
        Consumed::Consumed(self.into_inner())
    }

    ///Converts `self` into theEmpty state
    pub fn as_empty(self) -> Consumed<T> {
        Consumed::Empty(self.into_inner())
    }

    ///Maps over the contained value without changing the consumed state
    pub fn map<F, U>(self, f: F) -> Consumed<U>
        where F: FnOnce(T) -> U {
        match self {
            Consumed::Empty(x) => Consumed::Empty(f(x)),
            Consumed::Consumed(x) => Consumed::Consumed(f(x))
        }
    }

    ///Combines the Consumed flags from `self` and the result of `f`
    ///
    ///```
    /// # extern crate combine as pc;
    /// # use pc::*;
    /// # fn main() {
    /// //Parses a characther of string literal and handles the escaped characthers \\ and \" as \
    /// //and " respectively
    /// fn char(input: State<&str>) -> ParseResult<char, &str> {
    ///     let (c, input) = try!(satisfy(|c| c != '"').parse_state(input));
    ///     match c {
    ///         //Since the `char` parser has already consumed some of the input `combine` is used
    ///         //propagate the consumed state to the next part of the parser
    ///         '\\' => input.combine(|input| {
    ///             satisfy(|c| c == '"' || c == '\\')
    ///                 .map(|c| {
    ///                     match c {
    ///                         '"' => '"',
    ///                         '\\' => '\\',
    ///                         c => c
    ///                     }
    ///                 })
    ///                 .parse_state(input)
    ///             }),
    ///         _ => Ok((c, input))
    ///     }
    /// }
    /// let result = many(parser(char))
    ///     .parse(r#"abc\"\\"#);
    /// assert_eq!(result, Ok((r#"abc"\"#.to_string(), "")));
    /// }
    ///```
    pub fn combine<F, U, I>(self, f: F) -> ParseResult<U, I>
        where F: FnOnce(T) -> ParseResult<U, I>
            , I: Stream {
        match self {
            Consumed::Consumed(x) => {
                match f(x) {
                    Ok((v, Consumed::Empty(rest))) => Ok((v, Consumed::Consumed(rest))),
                    Err(Consumed::Empty(err)) => Err(Consumed::Consumed(err)),
                    y => y
                }
            }
            Consumed::Empty(x) => f(x)
        }
    }
}
///Struct which hold information about an error that occured at a specific position.
///Can hold multiple instances of `Error` if more that one error occured at the position.
pub struct ParseError<P: Stream> {
    ///The position where the error occured
    pub position: <P::Item as Positioner>::Position,
    ///A vector containing specific information on what errors occured at `position`
    pub errors: Vec<Error<P::Item, P::Range>>
}

impl <P: Positioner + Clone, S: Stream<Item=P>> ParseError<S> {
    
    pub fn new(position: P::Position, error: Error<S::Item, S::Range>) -> ParseError<S> {
        ParseError::from_errors(position, vec![error])
    }

    pub fn empty(position: P::Position) -> ParseError<S> {
        ParseError::from_errors(position, vec![])
    }

    pub fn from_errors(position: P::Position, errors: Vec<Error<P, S::Range>>) -> ParseError<S> {
        ParseError { position: position, errors: errors }
    }

    pub fn end_of_input(position: P::Position) -> ParseError<S> {
        ParseError::from_errors(position, vec![Error::end_of_input()])
    }

    pub fn add_message<M>(&mut self, message: M)
        where M: Into<Info<P, S::Range>> {
        self.add_error(Error::Message(message.into()));
    }

    pub fn add_error(&mut self, message: Error<P, S::Range>) {
        //Don't add duplicate errors
        if self.errors.iter().find(|msg| **msg == message).is_none() {
            self.errors.push(message);
        }
    }

    pub fn set_expected(&mut self, message: Info<P, S::Range>) {
        //Remove all other expected messages
        self.errors.retain(|e| match *e { Error::Expected(_) => false, _ => true });
        self.errors.push(Error::Expected(message));
    }

    pub fn merge(mut self, other: ParseError<S>) -> ParseError<S> {
        use std::cmp::Ordering;
        //Only keep the errors which occured after consuming the most amount of data
        match self.position.cmp(&other.position) {
            Ordering::Less => other,
            Ordering::Greater => self,
            Ordering::Equal => {
                for message in other.errors.into_iter() {
                    self.add_error(message);
                }
                self
            }
        }
    }
}

impl <S> StdError for ParseError<S>
    where S: Stream
        , S::Range: fmt::Display + fmt::Debug + Any
        , S::Item: fmt::Display + fmt::Debug + Any
        , <S::Item as Positioner>::Position: fmt::Display + fmt::Debug + Any {
    fn description(&self) -> &str { "parse error" }
}

impl <S> PartialEq for ParseError<S>
    where S: Stream
        , <S::Item as Positioner>::Position: PartialEq {
    fn eq(&self, other: &ParseError<S>) -> bool {
        self.position == other.position && self.errors == other.errors
    }
}

impl <S> fmt::Debug for ParseError<S>
    where S: Stream
        , S::Range: fmt::Debug
        , S::Item: fmt::Debug
        , <S::Item as Positioner>::Position: fmt::Debug {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ParseError {{ position: {:?}, errors: {:?} }}", self.position, self.errors)
    }
}

impl <S> fmt::Display for ParseError<S>
    where S: Stream
        , S::Item: fmt::Display
        , S::Range: fmt::Display
        , <S::Item as Positioner>::Position: fmt::Display {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        try!(writeln!(f, "Parse error at {}", self.position));

        //First print the token that we did not expect
        //There should really just be one unexpected message at this point though we print them
        //all to be safe
        let unexpected = self.errors.iter()
            .filter(|e| match **e { Error::Unexpected(_) => true, _ => false } );
        for error in unexpected {
            try!(writeln!(f, "{}", error));
        }

        //Then we print out all the things that were expected in a comma separated list
        //'Expected 'a', 'expression' or 'let'
        let expected_count = self.errors.iter()
            .filter(|e| match **e { Error::Expected(_) => true, _ => false } )
            .count();
        let mut i = 0;
        for error in self.errors.iter() {
            match *error {
                Error::Expected(ref message) => {
                    i += 1;
                    if i == 1 {
                        try!(write!(f, "Expected"));
                    }
                    else if i == expected_count {//Last expected message to be written
                        try!(write!(f, " or"));
                    }
                    else {
                        try!(write!(f, ","));
                    }
                    try!(write!(f, " '{}'", message));
                }
                _ => ()
            }
        }
        if expected_count != 0 {
            try!(writeln!(f, ""));
        }
        //If there are any generic messages we print them out last
        let messages = self.errors.iter()
            .filter(|e| match **e { Error::Message(_) | Error::Other(_) => true, _ => false } );
        for error in messages {
            try!(writeln!(f, "{}", error));
        }
        Ok(())
    }
}
impl fmt::Display for SourcePosition {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "line: {}, column: {}", self.line, self.column)
    }
}
impl <T: fmt::Display, R: fmt::Display> fmt::Display for Error<T, R> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Unexpected(ref c) => write!(f, "Unexpected token '{}'", c),
            Error::Expected(ref s) => write!(f, "Expected {}", s),
            Error::Message(ref msg) => write!(f, "{}", msg),
            Error::Other(ref err) => err.fmt(f)
        }
    }
}

///The `State<I>` struct keeps track of the current position in the stream `I`
#[derive(Clone, PartialEq)]
pub struct State<I>
    where I: Stream {
    pub position: <I::Item as Positioner>::Position,
    pub input: I
}

impl <I> fmt::Debug for State<I>
    where I: Stream + fmt::Debug
        , <I::Item as Positioner>::Position: fmt::Debug {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "State {{ position: {:?}, input: {:?} }}", self.position, self.input)
    }
}

impl <I: Stream> State<I> {
    pub fn new(input: I) -> State<I> {
        State { position: <I::Item as Positioner>::start(), input: input }
    }

    pub fn as_empty(&self) -> State<I> {
        State { position: self.position.clone(), input: self.input.clone() }
    }

    ///`uncons` is the most general way of extracting and item from a stream
    ///It takes a function `f` as argument which should update the position
    ///according to the item that was extracted
    ///Usually you want to use `uncons_char` instead which works directly on character streams
    pub fn uncons(self) -> ParseResult<I::Item, I> {
        let State { mut position, input, .. } = self;
        match input.uncons() {
            Ok((c, input)) => {
                c.update(&mut position);
                Ok((c, Consumed::Consumed(State { position: position, input: input })))
            }
            Err(err) => Err(Consumed::Empty(ParseError::new(position, err)))
        }
    }
    pub fn update(mut self, i: I::Item, rest: I) -> ParseResult<I::Item, I> {
        i.update(&mut self.position);
        self.input = rest;
        Ok((i, Consumed::Consumed(self)))
    }
}

///A type alias over the specific `Result` type used by parsers to indicate wether they were
///successful or not.
///`O` is the type that is output on success
///`I` is the specific stream type used in the parser
///`T` is the item type of `I`, this parameter will be removed once type declarations are allowed
///to have trait bounds
pub type ParseResult<O, I> = Result<(O, Consumed<State<I>>), Consumed<ParseError<I>>>;

///A stream is a sequence of items that can be extracted one by one
pub trait Stream : Clone {
    type Item: Positioner + Clone;
    type Range: Positioner + Clone;
    ///Takes a stream and removes its first item, yielding the item and the rest of the elements
    ///Returns `Err` when no more elements could be retrieved
    fn uncons(self) -> Result<(Self::Item, Self), Error<Self::Item, Self::Range>>;
}

impl <'a> Stream for &'a str {
    type Item = char;
    type Range = &'a str;
    fn uncons(self) -> Result<(char, &'a str), Error<char, &'a str>> {
        match self.chars().next() {
            Some(c) => Ok((c, &self[c.len_utf8()..])),
            None => Err(Error::end_of_input())
        }
    }
}

impl <'a, T> Stream for &'a [T]
    where T: Positioner + Copy {
    type Item = T;
    type Range = &'a [T];
    fn uncons(self) -> Result<(T, &'a [T]), Error<T, &'a [T]>> {
        if self.len() > 0 {
            Ok((self[0], &self[1..]))
        }
        else {
            Err(Error::end_of_input())
        }
    }
}

///Wrapper around iterators which allows them to be treated as a stream.
///Returned by `from_iter`.
#[derive(Clone, Debug)]
pub struct IteratorStream<I>(I)
    where I: Iterator + Clone;


///Converts an `Iterator` into a stream.
pub fn from_iter<I>(iter: I) -> IteratorStream<I>
    where I: Iterator + Clone {
    IteratorStream(iter)
}

impl <I: Iterator + Clone> Stream for IteratorStream<I>
    where I::Item: Positioner + Clone {
    type Item = I::Item;
    type Range = I::Item;
    fn uncons(mut self) -> Result<(I::Item, Self), Error<I::Item, I::Item>> {
        match self.0.next() {
            Some(x) => Ok((x, self)),
            None => Err(Error::end_of_input())
        }
    }
}

///`Positioner` represents the operations needed to update a position given an item from the stream
///When implementing stream for custom token type this must be implemented for that token to allow
///the position to be updated
pub trait Positioner: PartialEq {
    type Position: Clone + Ord;
    ///Creates a start position
    fn start() -> Self::Position;
    ///Updates the position given that `self` has been taken from the stream
    fn update(&self, position: &mut Self::Position);
}
impl <'a, T: ?Sized> Positioner for &'a T
    where T: Positioner {
    type Position = <T as Positioner>::Position;
    fn start() -> <T as Positioner>::Position {
        <T as Positioner>::start()
    }
    fn update(&self, position: &mut <T as Positioner>::Position) {
        (*self).update(position)
    }
}
impl <T> Positioner for [T]
    where T: Positioner {
    type Position = <T as Positioner>::Position;
    fn start() -> <T as Positioner>::Position {
        <T as Positioner>::start()
    }
    fn update(&self, position: &mut <T as Positioner>::Position) {
        for t in self {
            t.update(position);
        }
    }
}
impl Positioner for str {
    type Position = SourcePosition;
    fn start() -> SourcePosition {
        char::start()
    }
    fn update(&self, position: &mut SourcePosition) {
        for t in self.chars() {
            t.update(position);
        }
    }
}

impl Positioner for char {
    type Position = SourcePosition;
    fn start() -> SourcePosition {
        SourcePosition { line: 1, column: 1 }
    }
    fn update(&self, position: &mut SourcePosition) {
        position.column += 1;
        if *self == '\n' {
            position.column = 1;
            position.line += 1;
        }
    }
}

impl Positioner for u8 {
    type Position = BytePosition;

    fn start() -> BytePosition {
        BytePosition { position: 0 }
    }

    fn update(&self, b: &mut BytePosition) {
        b.position += 1;
    }
}

///By implementing the `Parser` trait a type says that it can be used to parse an input stream into
///the type `Output`.
///
///All methods have a default implementation but there needs to be at least an implementation of
///`parse_state` or`parse_lazy`. If `parse_ok` is implemented an implementation of `add_error` is
///also recommended to improve error reporting.
pub trait Parser {
    ///A type implementing the `Stream` trait which is the specific type
    ///that is parsed.
    type Input: Stream;
    ///The type which is returned when the parsing is successful.
    type Output;

    ///Entrypoint of the parser
    ///Takes some input and tries to parse it returning a `ParseResult`
    fn parse(&mut self, input: Self::Input) -> Result<(Self::Output, Self::Input), ParseError<Self::Input>> {
        match self.parse_state(State::new(input)) {
            Ok((v, state)) => Ok((v, state.into_inner().input)),
            Err(error) => Err(error.into_inner())
        }
    }
    ///Parses using the state `input` by calling Stream::uncons one or more times
    ///On success returns `Ok((value, new_state))` on failure it returns `Err(error)`
    fn parse_state(&mut self, input: State<Self::Input>) -> ParseResult<Self::Output, Self::Input> {
        let mut result = self.parse_lazy(input.clone());
        if let Err(Consumed::Empty(ref mut error)) = result {
            if let Ok((t, _)) = input.input.uncons() {
                error.add_error(Error::Unexpected(Info::Token(t)));
            }
            self.add_error(error);
        }
        result
    }

    ///Specialized version of parse_state where the parser does not need to add an error to the
    ///`ParseError` when it does not consume any input before encountering the error.
    ///Instead the error can be added later through the `add_error` method
    fn parse_lazy(&mut self, input: State<Self::Input>) -> ParseResult<Self::Output, Self::Input> {
        self.parse_state(input)
    }

    ///Adds the first error that would normally be returned by this parser if it failed
    fn add_error(&mut self, _error: &mut ParseError<Self::Input>) {
    }
}
impl <'a, I, O, P: ?Sized> Parser for &'a mut P 
    where I: Stream, P: Parser<Input=I, Output=O> {
    type Input = I;
    type Output = O;
    fn parse_state(&mut self, input: State<I>) -> ParseResult<O, I> {
        (**self).parse_state(input)
    }
    fn parse_lazy(&mut self, input: State<I>) -> ParseResult<O, I> {
        (**self).parse_lazy(input)
    }
    fn add_error(&mut self, error: &mut ParseError<Self::Input>) {
        (**self).add_error(error)
    }
}
impl <I, O, P: ?Sized> Parser for Box<P> 
    where I: Stream, P: Parser<Input=I, Output=O> {
    type Input = I;
    type Output = O;
    fn parse_state(&mut self, input: State<I>) -> ParseResult<O, I> {
        (**self).parse_state(input)
    }
    fn parse_lazy(&mut self, input: State<I>) -> ParseResult<O, I> {
        (**self).parse_lazy(input)
    }
    fn add_error(&mut self, error: &mut ParseError<Self::Input>) {
        (**self).add_error(error)
    }
}
