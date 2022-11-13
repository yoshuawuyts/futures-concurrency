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
