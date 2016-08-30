/// `alt!(I -> IResult<I,O> | I -> IResult<I,O> | ... | I -> IResult<I,O> ) => I -> IResult<I, O>`
/// try a list of parsers, return the result of the first successful one
///
/// If one of the parser returns Incomplete, alt will return Incomplete, to retry
/// once you get more input. Note that it is better for performance to know the
/// minimum size of data you need before you get into alt.
///
/// ```
/// # #[macro_use] extern crate nom;
/// # use nom::IResult::Done;
/// # fn main() {
///  named!( test, alt!( tag!( "abcd" ) | tag!( "efgh" ) ) );
///  let r1 = test(b"abcdefgh");
///  assert_eq!(r1, Done(&b"efgh"[..], &b"abcd"[..]));
///  let r2 = test(&b"efghijkl"[..]);
///  assert_eq!(r2, Done(&b"ijkl"[..], &b"efgh"[..]));
///  # }
/// ```
///
/// There is another syntax for alt allowing a block to manipulate the result:
///
/// ```
/// # #[macro_use] extern crate nom;
/// # use nom::IResult::Done;
/// # fn main() {
///     #[derive(Debug,PartialEq,Eq)]
///     enum Tagged {
///       Abcd,
///       Efgh,
///       Took(usize)
///     }
///     named!(test<Tagged>, alt!(
///         tag!("abcd") => { |_|          Tagged::Abcd }
///       | tag!("efgh") => { |_|          Tagged::Efgh }
///       | take!(5)     => { |res: &[u8]| Tagged::Took(res.len()) } // the closure takes the result as argument if the parser is successful
///     ));
///     let r1 = test(b"abcdefgh");
///     assert_eq!(r1, Done(&b"efgh"[..], Tagged::Abcd));
///     let r2 = test(&b"efghijkl"[..]);
///     assert_eq!(r2, Done(&b"ijkl"[..], Tagged::Efgh));
///     let r3 = test(&b"mnopqrst"[..]);
///     assert_eq!(r3, Done(&b"rst"[..],  Tagged::Took(5)));
/// # }
/// ```
///
/// **BE CAREFUL** there is a case where the behaviour of `alt!` can be confusing:
///
/// when the alternatives have different lengths, like this case:
///
/// ```ignore
///  named!( test, alt!( tag!( "abcd" ) | tag!( "ef" ) | tag!( "ghi" ) | tag!( "kl" ) ) );
/// ```
///
/// With this parser, if you pass `"abcd"` as input, the first alternative parses it correctly,
/// but if you pass `"efg"`, the first alternative will return `Incomplete`, since it needs an input
/// of 4 bytes. This behaviour of `alt!` is expected: if you get a partial input that isn't matched
/// by the first alternative, but would match if the input was complete, you want `alt!` to indicate
/// that it cannot decide with limited information.
///
/// There are two ways to fix this behaviour. The first one consists in ordering the alternatives
/// by size, like this:
///
/// ```ignore
///  named!( test, alt!( tag!( "ef" ) | tag!( "kl") | tag!( "ghi" ) | tag!( "abcd" ) ) );
/// ```
///
/// With this solution, the largest alternative will be tested last.
///
/// The other solution uses the `complete!` combinator, which transforms an `Incomplete` in an
/// `Error`. If one of the alternatives returns `Incomplete` but is wrapped by `complete!`,
/// `alt!` will try the next alternative. This is useful when you know that
/// you will not get partial input:
///
/// ```ignore
///  named!( test,
///    alt!(
///      complete!( tag!( "abcd" ) ) |
///      complete!( tag!( "ef"   ) ) |
///      complete!( tag!( "ghi"  ) ) |
///      complete!( tag!( "kl"   ) )
///    )
///  );
/// ```
///
/// If you want the `complete!` combinator to be applied to all rules then use the convenience
/// `alt_complete!` macro (see below).
///
/// This behaviour of `alt!` can get especially confusing if multiple alternatives have different
/// sizes but a common prefix, like this:
///
/// ```ignore
///  named!( test, alt!( tag!( "abcd" ) | tag!( "ab" ) | tag!( "ef" ) ) );
/// ```
///
/// in that case, if you order by size, passing `"abcd"` as input will always be matched by the
/// smallest parser, so the solution using `complete!` is better suited.
///
/// You can also nest multiple `alt!`, like this:
///
/// ```ignore
///  named!( test,
///    alt!(
///      preceded!(
///        tag!("ab"),
///        alt!(
///          tag!( "cd" ) |
///          eof
///        )
///      )
///    | tag!( "ef" )
///    )
///  );
/// ```
///
///  `preceded!` will first parse `"ab"` then, if successful, try the alternatives "cd",
///  or empty input (End Of File). If none of them work, `preceded!` will fail and
///  "ef" will be tested.
///
#[macro_export]
macro_rules! alt (
  ($i:expr, $($rest:tt)*) => (
    {
      alt_parser!($i, $($rest)*)
    }
  );
);

/// Internal parser, do not use directly
#[doc(hidden)]
#[macro_export]
macro_rules! alt_parser (
  ($i:expr, $e:ident | $($rest:tt)*) => (
    alt_parser!($i, call!($e) | $($rest)*);
  );

  ($i:expr, $subrule:ident!( $($args:tt)*) | $($rest:tt)*) => (
    {
      let res = $subrule!($i, $($args)*);
      match res {
        $crate::IResult::Done(_,_)     => res,
        $crate::IResult::Incomplete(_) => res,
        _                              => alt_parser!($i, $($rest)*)
      }
    }
  );

  ($i:expr, $subrule:ident!( $($args:tt)* ) => { $gen:expr } | $($rest:tt)+) => (
    {
      match $subrule!( $i, $($args)* ) {
        $crate::IResult::Done(i,o)     => $crate::IResult::Done(i,$gen(o)),
        $crate::IResult::Incomplete(x) => $crate::IResult::Incomplete(x),
        $crate::IResult::Error(_)      => {
          alt_parser!($i, $($rest)*)
        }
      }
    }
  );

  ($i:expr, $e:ident => { $gen:expr } | $($rest:tt)*) => (
    alt_parser!($i, call!($e) => { $gen } | $($rest)*);
  );

  ($i:expr, $e:ident => { $gen:expr }) => (
    alt_parser!($i, call!($e) => { $gen });
  );

  ($i:expr, $subrule:ident!( $($args:tt)* ) => { $gen:expr }) => (
    {
      match $subrule!( $i, $($args)* ) {
        $crate::IResult::Done(i,o)     => $crate::IResult::Done(i,$gen(o)),
        $crate::IResult::Incomplete(x) => $crate::IResult::Incomplete(x),
        $crate::IResult::Error(_)      => {
          alt_parser!($i)
        }
      }
    }
  );

  ($i:expr, $e:ident) => (
    alt_parser!($i, call!($e));
  );

  ($i:expr, $subrule:ident!( $($args:tt)*)) => (
    {
      match $subrule!( $i, $($args)* ) {
        $crate::IResult::Done(i,o)     => $crate::IResult::Done(i,o),
        $crate::IResult::Incomplete(x) => $crate::IResult::Incomplete(x),
        $crate::IResult::Error(_)      => {
          alt_parser!($i)
        }
      }
    }
  );

  ($i:expr) => (
    $crate::IResult::Error(error_position!($crate::ErrorKind::Alt,$i))
  );
);

/// This is a combination of the `alt!` and `complete!` combinators. Rather
/// than returning `Incomplete` on partial input, `alt_complete!` will try the
/// next alternative in the chain. You should use this only if you know you
/// will not receive partial input for the rules you're trying to match (this
/// is almost always the case for parsing programming languages).
#[macro_export]
macro_rules! alt_complete (
  // Recursive rules (must include `complete!` around the head)

  ($i:expr, $e:ident | $($rest:tt)*) => (
    alt_complete!($i, complete!(call!($e)) | $($rest)*);
  );

  ($i:expr, $subrule:ident!( $($args:tt)*) | $($rest:tt)*) => (
    {
      let res = complete!($i, $subrule!($($args)*));
      match res {
        $crate::IResult::Done(_,_) => res,
        _ => alt_complete!($i, $($rest)*),
      }
    }
  );

  ($i:expr, $subrule:ident!( $($args:tt)* ) => { $gen:expr } | $($rest:tt)+) => (
    {
      match complete!($i, $subrule!($($args)*)) {
        $crate::IResult::Done(i,o) => $crate::IResult::Done(i,$gen(o)),
        _ => alt_complete!($i, $($rest)*),
      }
    }
  );

  ($i:expr, $e:ident => { $gen:expr } | $($rest:tt)*) => (
    alt_complete!($i, complete!(call!($e)) => { $gen } | $($rest)*);
  );

  // Tail (non-recursive) rules

  ($i:expr, $e:ident => { $gen:expr }) => (
    alt_complete!($i, call!($e) => { $gen });
  );

  ($i:expr, $subrule:ident!( $($args:tt)* ) => { $gen:expr }) => (
    alt_parser!($i, $subrule!($($args)*) => { $gen })
  );

  ($i:expr, $e:ident) => (
    alt_complete!($i, call!($e));
  );

  ($i:expr, $subrule:ident!( $($args:tt)*)) => (
    alt_parser!($i, $subrule!($($args)*))
  );
);

/// `switch!(I -> IResult<I,P>, P => I -> IResult<I,O> | ... | P => I -> IResult<I,O> ) => I -> IResult<I, O>`
/// choose the next parser depending on the result of the first one, if successful,
/// and returns the result of the second parser
///
/// ```
/// # #[macro_use] extern crate nom;
/// # use nom::IResult::{Done,Error};
/// # #[cfg(feature = "verbose-errors")]
/// # use nom::Err::{Position, NodePosition};
/// # use nom::ErrorKind;
/// # fn main() {
///  named!(sw,
///    switch!(take!(4),
///      b"abcd" => tag!("XYZ") |
///      b"efgh" => tag!("123")
///    )
///  );
///
///  let a = b"abcdXYZ123";
///  let b = b"abcdef";
///  let c = b"efgh123";
///  let d = b"blah";
///
///  assert_eq!(sw(&a[..]), Done(&b"123"[..], &b"XYZ"[..]));
///  assert_eq!(sw(&b[..]), Error(error_node_position!(ErrorKind::Switch, &b"abcdef"[..],
///    error_position!(ErrorKind::Tag, &b"ef"[..]))));
///  assert_eq!(sw(&c[..]), Done(&b""[..], &b"123"[..]));
///  assert_eq!(sw(&d[..]), Error(error_position!(ErrorKind::Switch, &b"blah"[..])));
///  # }
/// ```
///
/// Due to limitations in Rust macros, it is not possible to have simple functions on the right hand
/// side of pattern, like this:
///
/// ```ignore
///  named!(sw,
///    switch!(take!(4),
///      b"abcd" => tag!("XYZ") |
///      b"efgh" => tag!("123")
///    )
///  );
/// ```
///
/// If you want to pass your own functions instead, you can use the `call!` combinator as follows:
///
/// ```ignore
///  named!(xyz, tag!("XYZ"));
///  named!(num, tag!("123"));
///  named!(sw,
///    switch!(take!(4),
///      b"abcd" => call!(xyz) |
///      b"efgh" => call!(num)
///    )
///  );
/// ```
///
#[macro_export]
macro_rules! switch (
  ($i:expr, $submac:ident!( $($args:tt)*), $($rest:tt)*) => (
    {
      switch_impl!($i, $submac!($($args)*), $($rest)*)
    }
  );
  ($i:expr, $e:ident, $($rest:tt)*) => (
    {
      switch_impl!($i, call!($e), $($rest)*)
    }
  );
);

/// Internal parser, do not use directly
#[doc(hidden)]
#[macro_export]
macro_rules! switch_impl (
  ($i:expr, $submac:ident!( $($args:tt)* ), $($p:pat => $subrule:ident!( $($args2:tt)* ))|* ) => (
    {
      match $submac!($i, $($args)*) {
        $crate::IResult::Error(e)      => $crate::IResult::Error(error_node_position!(
            $crate::ErrorKind::Switch, $i, e
        )),
        $crate::IResult::Incomplete(i) => $crate::IResult::Incomplete(i),
        $crate::IResult::Done(i, o)    => {
          match o {
            $($p => match $subrule!(i, $($args2)*) {
              $crate::IResult::Error(e) => $crate::IResult::Error(error_node_position!(
                  $crate::ErrorKind::Switch, $i, e
              )),
              a => a,
            }),*,
            _    => $crate::IResult::Error(error_position!($crate::ErrorKind::Switch,$i))
          }
        }
      }
    }
  );
);

#[macro_export]
macro_rules! permutation (
  ($i:expr, $($rest:tt)*) => (
    {
      use util::HexDisplay;
      let mut res   = permutation_init!((), $($rest)*);
      let mut input = $i;
      let mut error = None;

      loop {
        let mut all_done = true;
        println!("res = {:?}, input:\n{}", res, input.to_hex(16));
        permutation_iterator!(0, input, all_done, res, $($rest)*);

        //if we reach that part, it means none of the parsers were able to read anything
        if !all_done {
          error = Some(error_position!($crate::ErrorKind::Permutation, input));
        }
        break;
      }

      //FIXME: handle incomplete
      if let Some(e) = error {
        $crate::IResult::Error(e)
      } else {
        let unwrapped_res = permutation_unwrap!(0, (), res, $($rest)*);
        $crate::IResult::Done(input, unwrapped_res)
      }
    }
  );
);


#[doc(hidden)]
#[macro_export]
macro_rules! permutation_init (
  ((), $e:ident, $($rest:tt)*) => (
    permutation_init!((None), $($rest)*);
  );
  ((), $submac:ident!( $($args:tt)* ), $($rest:tt)*) => (
    permutation_init!((None), $($rest)*);
  );
  (($($parsed:tt)*), $e:ident, $($rest:tt)*) => (
    permutation_init!(($($parsed)* , None), $($rest)*);
  );
  (($($parsed:tt)*), $submac:ident!( $($args:tt)* ), $($rest:tt)*) => (
    permutation_init!(($($parsed)* , None), $($rest)*);
  );
  (($($parsed:tt)*), $e:ident) => (
    ($($parsed)* , None)
  );
  (($($parsed:tt)*), $submac:ident!( $($args:tt)* )) => (
    ($($parsed)* , None)
  );
);

#[doc(hidden)]
#[macro_export]
macro_rules! succ (
  (0, $submac:ident ! ($($rest:tt)*)) => ($submac!(1, $($rest)*));
  (1, $submac:ident ! ($($rest:tt)*)) => ($submac!(2, $($rest)*));
  (2, $submac:ident ! ($($rest:tt)*)) => ($submac!(3, $($rest)*));
  (3, $submac:ident ! ($($rest:tt)*)) => ($submac!(4, $($rest)*));
);

#[doc(hidden)]
#[macro_export]
macro_rules! permutation_unwrap (
  ($it:tt,  (), $res:expr, $submac:ident!( $($args:tt)* ), $($rest:tt)*) => (
    succ!($it, permutation_unwrap!(($res.$it.unwrap()), $res, $($rest)*));
  );
  ($it:tt, ($($parsed:tt)*), $res:expr, $e:ident, $($rest:tt)*) => (
    succ!($it, permutation_unwrap!(($($parsed)* , $res.$it.unwrap()), $res, $($rest)*));
  );
  ($it:tt, ($($parsed:tt)*), $res:expr, $submac:ident!( $($args:tt)* ), $($rest:tt)*) => (
    succ!($it, permutation_unwrap!(($($parsed)* , $res.$it.unwrap()), $res, $($rest)*));
  );
  ($it:tt, ($($parsed:tt)*), $res:expr, $e:ident) => (
    ($($parsed)* , $res.$it.unwrap())
  );
  ($it:tt, ($($parsed:tt)*), $res:expr, $submac:ident!( $($args:tt)* )) => (
    ($($parsed)* , $res.$it.unwrap())
  );
);

#[doc(hidden)]
#[macro_export]
macro_rules! permutation_iterator (
  ($it:tt,$i:expr, $all_done:expr, $res:expr, $e:ident, $($rest:tt)*) => (
    permutation_iterator!($it, $i, $all_done, $res, call!($e), $($rest)*);
  );
  ($it:tt, $i:expr, $all_done:expr, $res:expr, $submac:ident!( $($args:tt)* ), $($rest:tt)*) => {
    if ($res).$it == None {
      match $submac!($i, $($args)*) {
        //$crate::IResult::Error(e)      => $crate::IResult::Error(e),
        //$crate::IResult::Incomplete(i) => $crate::IResult::Incomplete(i),
        $crate::IResult::Done(i,o)     => {
          $i = i;
          ($res).$it = Some(o);
          continue;
        },
        _ => {
          $all_done = false;
        }
      };
    }
    succ!($it, permutation_iterator!($i, $all_done, $res, $($rest)*));
  };
  ($it:tt,$i:expr, $all_done:expr, $res:expr, $e:ident) => (
    permutation_iterator!($it, $i, $all_done, $res, call!($e));
  );
  ($it:tt, $i:expr, $all_done:expr, $res:expr, $submac:ident!( $($args:tt)* )) => {
    if ($res).$it == None {
      match $submac!($i, $($args)*) {
        //$crate::IResult::Error(e)      => $crate::IResult::Error(e),
        //$crate::IResult::Incomplete(i) => $crate::IResult::Incomplete(i),
        $crate::IResult::Done(i,o)     => {
          $i = i;
          ($res).$it = Some(o);
          continue;
        },
        _ => {
          $all_done = false;
        }
      };
    }
  };
);

#[cfg(test)]
mod tests {
  use internal::{Needed,IResult};
  use internal::IResult::*;
  use util::ErrorKind;

  // reproduce the tag and take macros, because of module import order
  macro_rules! tag (
    ($i:expr, $inp: expr) => (
      {
        #[inline(always)]
        fn as_bytes<T: $crate::AsBytes>(b: &T) -> &[u8] {
          b.as_bytes()
        }

        let expected = $inp;
        let bytes    = as_bytes(&expected);

        tag_bytes!($i,bytes)
      }
    );
  );

  macro_rules! tag_bytes (
    ($i:expr, $bytes: expr) => (
      {
        use std::cmp::min;
        let len = $i.len();
        let blen = $bytes.len();
        let m   = min(len, blen);
        let reduced = &$i[..m];
        let b       = &$bytes[..m];

        let res: $crate::IResult<_,_> = if reduced != b {
          $crate::IResult::Error(error_position!($crate::ErrorKind::Tag, $i))
        } else if m < blen {
          $crate::IResult::Incomplete($crate::Needed::Size(blen))
        } else {
          $crate::IResult::Done(&$i[blen..], reduced)
        };
        res
      }
    );
  );

  macro_rules! take(
    ($i:expr, $count:expr) => (
      {
        let cnt = $count as usize;
        let res:$crate::IResult<&[u8],&[u8]> = if $i.len() < cnt {
          $crate::IResult::Incomplete($crate::Needed::Size(cnt))
        } else {
          $crate::IResult::Done(&$i[cnt..],&$i[0..cnt])
        };
        res
      }
    );
  );

#[test]
  fn alt() {
    fn work(input: &[u8]) -> IResult<&[u8],&[u8], &'static str> {
      Done(&b""[..], input)
    }

    #[allow(unused_variables)]
    fn dont_work(input: &[u8]) -> IResult<&[u8],&[u8],&'static str> {
      Error(error_code!(ErrorKind::Custom("abcd")))
    }

    fn work2(input: &[u8]) -> IResult<&[u8],&[u8], &'static str> {
      Done(input, &b""[..])
    }

    fn alt1(i:&[u8]) ->  IResult<&[u8],&[u8], &'static str> {
      alt!(i, dont_work | dont_work)
    }
    fn alt2(i:&[u8]) ->  IResult<&[u8],&[u8], &'static str> {
      alt!(i, dont_work | work)
    }
    fn alt3(i:&[u8]) ->  IResult<&[u8],&[u8], &'static str> {
      alt!(i, dont_work | dont_work | work2 | dont_work)
    }
    //named!(alt1, alt!(dont_work | dont_work));
    //named!(alt2, alt!(dont_work | work));
    //named!(alt3, alt!(dont_work | dont_work | work2 | dont_work));

    let a = &b"abcd"[..];
    assert_eq!(alt1(a), Error(error_position!(ErrorKind::Alt, a)));
    assert_eq!(alt2(a), Done(&b""[..], a));
    assert_eq!(alt3(a), Done(a, &b""[..]));

    named!(alt4, alt!(tag!("abcd") | tag!("efgh")));
    let b = &b"efgh"[..];
    assert_eq!(alt4(a), Done(&b""[..], a));
    assert_eq!(alt4(b), Done(&b""[..], b));

    // test the alternative syntax
    named!(alt5<bool>, alt!(tag!("abcd") => { |_| false } | tag!("efgh") => { |_| true }));
    assert_eq!(alt5(a), Done(&b""[..], false));
    assert_eq!(alt5(b), Done(&b""[..], true));

  }

  #[test]
  fn alt_incomplete() {
    named!(alt1, alt!(tag!("a") | tag!("bc") | tag!("def")));

    let a = &b""[..];
    assert_eq!(alt1(a), Incomplete(Needed::Size(1)));
    let a = &b"b"[..];
    assert_eq!(alt1(a), Incomplete(Needed::Size(2)));
    let a = &b"bcd"[..];
    assert_eq!(alt1(a), Done(&b"d"[..], &b"bc"[..]));
    let a = &b"cde"[..];
    assert_eq!(alt1(a), Error(error_position!(ErrorKind::Alt, a)));
    let a = &b"de"[..];
    assert_eq!(alt1(a), Incomplete(Needed::Size(3)));
    let a = &b"defg"[..];
    assert_eq!(alt1(a), Done(&b"g"[..], &b"def"[..]));
  }

  #[test]
  fn alt_complete() {
    named!(ac<&[u8], &[u8]>,
      alt_complete!(tag!("abcd") | tag!("ef") | tag!("ghi") | tag!("kl"))
    );

    let a = &b""[..];
    assert_eq!(ac(a), Incomplete(Needed::Size(2)));
    let a = &b"ef"[..];
    assert_eq!(ac(a), Done(&b""[..], &b"ef"[..]));
    let a = &b"cde"[..];
    assert_eq!(ac(a), Error(error_position!(ErrorKind::Alt, a)));
  }

  #[test]
  fn switch() {
    named!(sw,
      switch!(take!(4),
        b"abcd" => take!(2) |
        b"efgh" => take!(4)
      )
    );

    let a = &b"abcdefgh"[..];
    assert_eq!(sw(a), Done(&b"gh"[..], &b"ef"[..]));

    let b = &b"efghijkl"[..];
    assert_eq!(sw(b), Done(&b""[..], &b"ijkl"[..]));
    let c = &b"afghijkl"[..];
    assert_eq!(sw(c), Error(error_position!(ErrorKind::Switch, &b"afghijkl"[..])));
  }

  #[test]
  fn permutation() {
    //trace_macros!(true);
    named!(perm<(&[u8], &[u8], &[u8])>,
      permutation!(tag!("abcd"), tag!("efg"), tag!("hi"))
    );
    //trace_macros!(false);

    let expected = (&b"abcd"[..], &b"efg"[..], &b"hi"[..]);

    let a = &b"abcdefghijk"[..];
    assert_eq!(perm(a), Done(&b"jk"[..], expected));
    let b = &b"efgabcdhijk"[..];
    assert_eq!(perm(b), Done(&b"jk"[..], expected));
    let c = &b"hiefgabcdjk"[..];
    assert_eq!(perm(c), Done(&b"jk"[..], expected));

    let d = &b"efgxyzabcdefghi"[..];
    assert_eq!(perm(d), Error(error_position!(ErrorKind::Permutation, &b"xyzabcdefghi"[..])));

    /*
    let e = &b"efgabc"[..];
    assert_eq!(perm(e), Incomplete(Needed::Size(4)));
    */
  }
}
