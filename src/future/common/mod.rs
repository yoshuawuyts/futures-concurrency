mod array;
mod vec;
pub(crate) use array::{CombinatorArray, CombinatorBehaviorArray};
pub(crate) use vec::{CombinatorBehaviorVec, CombinatorVec};

#[derive(Debug)]
pub enum ReturnOrStore<T, U> {
    Return(T),
    Store(U),
}
