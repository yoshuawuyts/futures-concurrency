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

/// Generate the `match` conditions inside the main polling body. This macro
/// chooses a random starting point on each call to the given method, making
/// it "fair".
///
/// The way this algorithm works is: we generate a random number between 0 and
/// the length of the tuple we have. This number determines which element we
/// start with. All other cases are mapped as `r + index`, and after we have the
/// first one, we'll sequentially iterate over all others. The starting point of
/// the stream is random, but the iteration order of all others is not.
// NOTE(yosh): this macro monstrosity is needed so we can increment each `else
// if` branch with + 1. When RFC 3086 becomes available to us, we can replace
// this with `${index($F)}` to get the current iteration.
//
// # References
// - https://twitter.com/maybewaffle/status/1588426440835727360
// - https://twitter.com/Veykril/status/1588231414998335490
// - https://rust-lang.github.io/rfcs/3086-macro-metavar-expr.html
macro_rules! gen_conditions {
    // Base condition, setup the depth counter.
    ($LEN:expr, $i:expr, $r:expr, $this:expr, $cx:expr, $method:ident, $(($F_index: expr; $F:ident, { $($arms:pat => $foo:expr,)* }))*) => {
        $(
            if $i == ($r + $F_index).wrapping_rem($LEN) {
                match unsafe { Pin::new_unchecked(&mut $this.$F) }.$method($cx) {
                    $($arms => $foo,)*
                }
            }
        )*
    }
}
pub(crate) use gen_conditions;
