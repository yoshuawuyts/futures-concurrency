/// Compute the number of permutations for a number
/// during compilation.
pub(crate) const fn permutations(mut num: u32) -> u32 {
    let mut total = 1;
    loop {
        total *= num;
        num -= 1;
        if num == 0 {
            break total;
        }
    }
}

/// Calculate the number of tuples currently being operated on.
macro_rules! tuple_len {
    (@count_one $F:ident) => (1);
    ($($F:ident,)*) => (0 $(+ crate::utils::tuple_len!(@count_one $F))*);
}
pub(crate) use tuple_len;

/// Generate the `match` conditions inside the main polling body. This macro
/// chooses a random starting point on each call to the given method, making
/// it "fair".
///
/// The way this algorithm works is: we generate a random number between 0 and
/// the lenght of the tuple we have. This number determines which element we
/// start with. All other cases are mapped as `r + index`, and after we have the
/// first one, we'll sequentially iterate over all others. The starting point of
/// the stream is random, but the iteration order of all others is not.
// NOTE(yosh): this macro monstrocity is needed so we can increment each `else
// if` branch with + 1. When RFC 3086 becomes available to us, we can replace
// this with `${index($F)}` to get the current iteration.
//
// # References
// - https://twitter.com/maybewaffle/status/1588426440835727360
// - https://twitter.com/Veykril/status/1588231414998335490
// - https://rust-lang.github.io/rfcs/3086-macro-metavar-expr.html
macro_rules! gen_conditions {
    // Generate an if-block, and keep iterating.
    (@inner $LEN:expr, $i:expr, $r:expr, $this:expr, $cx:expr, $counter:expr, $method:ident, {$($arms:pat => $foo:expr,)*}, $F:ident, $($rest:ident,)*) => {
        if $i == ($r + $counter).wrapping_rem($LEN) {
            match unsafe { Pin::new_unchecked(&mut $this.$F) }.$method($cx) {
                $($arms => $foo,)*
            }
        }
        crate::utils::gen_conditions!(@inner $LEN, $i, $r, $this, $cx, $counter + 1, $method, {$($arms => $foo,)*}, $($rest,)*)
    };

    // End of recursion, nothing to do.
    (@inner $LEN:expr, $i:expr, $r:expr, $this:expr, $cx:expr, $counter:expr, $method:ident, {$($arms:pat => $foo:expr,)*},) => {};

    // Base condition, setup the depth counter.
    ($LEN:expr, $i:expr, $r:expr, $this:expr, $cx:expr, $method:ident, {$($arms:pat => $foo:expr,)*}, $($F:ident,)*) => {
        crate::utils::gen_conditions!(@inner $LEN, $i, $r, $this, $cx, 0, $method, {$($arms => $foo,)*}, $($F,)*)
    }
}
pub(crate) use gen_conditions;

/// Repeats a given macro for each element a tuple has, passing the iteration
/// number (aka the element index) to the macro called macro.
///
/// The given macro must receives the tuple index as its first argument.
///
/// The last argument to this macro should be a list of tokens of the tuple size
/// usually, this came in the form of `$($F)*`.
///
/// # Example:
///
/// ```ignore
/// macro_rules! reset {
///     ($tuple_index:tt, $tuple:expr) => { $tuple.$expr = false; }
/// }
///
/// let mut my_tuple = (true, 1, Some('a'));
/// tuple_for_each(reset (my_tuple) A B C);
///
/// assert!(my_tuple, (true, 1, None));
/// ```
// NOTE(matheus-consoli): the primary issue this macro attacks, tuple indexing,
// could also be solved by RFC 3086
//
// # References:
// - https://rust-lang.github.io/rfcs/3086-macro-metavar-expr.html#index-and-length
macro_rules! tuple_for_each {
    (@at 0 $mac:ident ($($args:tt),*) $ignore_popped:ident $($size:ident)*) => {
        $mac!(0, $($args),*);
        crate::utils::tuple_for_each!(@at 1 $mac ($($args),*) $($size)*);
    };

    (@at 1 $mac:ident ($($args:tt),*) $ignore_popped:ident $($size:ident)*) => {
        $mac!(1, $($args),*);
        crate::utils::tuple_for_each!(@at 2 $mac ($($args),*) $($size)*);
    };

    (@at 2 $mac:ident ($($args:tt),*) $ignore_popped:ident $($size:ident)*) => {
        $mac!(2, $($args),*);
        crate::utils::tuple_for_each!(@at 3 $mac ($($args),*) $($size)*);
    };

    (@at 3 $mac:ident ($($args:tt),*) $ignore_popped:ident $($size:ident)*) => {
        $mac!(3, $($args),*);
        crate::utils::tuple_for_each!(@at 4 $mac ($($args),*) $($size)*);
    };

    (@at 4 $mac:ident ($($args:tt),*) $ignore_popped:ident $($size:ident)*) => {
        $mac!(4, $($args),*);
        crate::utils::tuple_for_each!(@at 5 $mac ($($args),*) $($size)*);
    };

    (@at 5 $mac:ident ($($args:tt),*) $ignore_popped:ident $($size:ident)*) => {
        $mac!(5, $($args),*);
        crate::utils::tuple_for_each!(@at 6 $mac ($($args),*) $($size)*);
    };

    (@at 6 $mac:ident ($($args:tt),*) $ignore_popped:ident $($size:ident)*) => {
        $mac!(6, $($args),*);
        crate::utils::tuple_for_each!(@at 7 $mac ($($args),*) $($size)*);
    };

    (@at 7 $mac:ident ($($args:tt),*) $ignore_popped:ident $($size:ident)*) => {
        $mac!(7, $($args),*);
        crate::utils::tuple_for_each!(@at 8 $mac ($($args),*) $($size)*);
    };

    (@at 8 $mac:ident ($($args:tt),*) $ignore_popped:ident $($size:ident)*) => {
        $mac!(8, $($args),*);
        crate::utils::tuple_for_each!(@at 9 $mac ($($args),*) $($size)*);
    };

    (@at 9 $mac:ident ($($args:tt),*) $ignore_popped:ident $($size:ident)*) => {
        $mac!(9, $($args),*);
        crate::utils::tuple_for_each!(@at 10 $mac ($($args),*) $($size)*);
    };

    (@at 10 $mac:ident ($($args:tt),*) $ignore_popped:ident $($size:ident)*) => {
        $mac!(10, $($args),*);
        crate::utils::tuple_for_each!(@at 11 $mac ($($args),*) $($size)*);
    };

    (@at 11 $mac:ident ($($args:tt),*) $ignore_popped:ident $($size:ident)*) => {
        $mac!(11, $($args),*);
        crate::utils::tuple_for_each!(@at 12 $mac ($($args),*) $($size)*);
    };

    (@at 12 $mac:ident ($($args:tt),*) $ignore_popped:ident $($size:ident)*) => {
        $mac!(12, $($args),*);
    };

    // End of recursion, no more tuple elements to iterate
    (@at $idx:tt $mac:ident ($($args:tt),*) ) => {};

    ($mac:ident ($($args:tt),*) $($size:ident)*) => {
        crate::utils::tuple_for_each!(@at 0 $mac ($($args),*) $($size)*)
    };
}
pub(crate) use tuple_for_each;

#[test]
fn tuple_for_each_buffering() {
    use super::tuple_for_each;
    use core::fmt::Write;

    macro_rules! append_debug {
        ($idx:tt, $tuple:ident, $buffer:ident) => {
            write!($buffer, "{:?}", $tuple.$idx);
        };
    }

    let tuple = ('a', 1, (2,), Some(3), [4]);
    let mut buffer = String::new();

    tuple_for_each!(append_debug (tuple, buffer) T U P L E);

    assert_eq!("'a'1(2,)Some(3)[4]", buffer);
}

#[test]
fn tuple_for_each_double_odds() {
    use super::tuple_for_each;
    use core::fmt::Write;

    macro_rules! double_odds {
        ($idx:tt, $tuple:ident) => {
            if $tuple.$idx % 2 != 0 {
                $tuple.$idx *= 2;
            }
        };
    }

    let mut tuple = (1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12);

    tuple_for_each!(double_odds (tuple)  I G N O R E   M E   P L S E);

    assert_eq!((2, 2, 6, 4, 10, 6, 14, 8, 18, 10, 22, 12), tuple);
}
