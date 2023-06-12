use std::ops::{Deref, DerefMut};

use super::PollState;

/// The maximum number of entries that `PollStates` can store without
/// dynamic memory allocation.
///
/// The `Boxed` variant is the minimum size the data structure can have.
/// It consists of a boxed slice (=2 usizes) and space for the enum
/// tag (another usize because of padding), so 3 usizes.
/// The inline variant then consists of `3 * size_of(usize) - 2` entries.
/// Each entry is a byte and we subtract one byte for a length field,
/// and another byte for the enum tag.
///
/// ```txt
///                                 Boxed
///                                 vvvvv
/// tag
///  | <-------padding----> <--- Box<[T]>::len ---> <--- Box<[T]>::ptr --->
/// 00 01 02 03 04 05 06 07 08 09 10 11 12 13 14 15 16 17 18 19 20 21 22 23  <bytes
///  |  | <------------------- entries ----------------------------------->
/// tag |
///    len                          ^^^^^
///                                 Inline
/// ```
const MAX_INLINE_ENTRIES: usize = std::mem::size_of::<usize>() * 3 - 2;

pub(crate) enum PollVec {
    Inline(u8, [PollState; MAX_INLINE_ENTRIES]),
    Boxed(Box<[PollState]>),
}

impl PollVec {
    pub(crate) fn new(len: usize) -> Self {
        assert!(MAX_INLINE_ENTRIES <= u8::MAX as usize);

        if len <= MAX_INLINE_ENTRIES {
            Self::Inline(len as u8, Default::default())
        } else {
            // Make sure that we don't reallocate the vec's memory
            // during `Vec::into_boxed_slice()`.
            let mut states = Vec::new();
            debug_assert_eq!(states.capacity(), 0);
            states.reserve_exact(len);
            debug_assert_eq!(states.capacity(), len);
            states.resize(len, PollState::default());
            debug_assert_eq!(states.capacity(), len);
            Self::Boxed(states.into_boxed_slice())
        }
    }

    /// Get an iterator of indexes of all items which are "ready".
    pub(crate) fn ready_indexes(&self) -> impl Iterator<Item = usize> + '_ {
        self.iter()
            .cloned()
            .enumerate()
            .filter(|(_, state)| state.is_ready())
            .map(|(i, _)| i)
    }

    /// Get an iterator of indexes of all items which are "pending".
    #[allow(unused)]
    pub(crate) fn pending_indexes(&self) -> impl Iterator<Item = usize> + '_ {
        self.iter()
            .cloned()
            .enumerate()
            .filter(|(_, state)| state.is_pending())
            .map(|(i, _)| i)
    }
}

impl Deref for PollVec {
    type Target = [PollState];

    fn deref(&self) -> &Self::Target {
        match self {
            PollVec::Inline(len, states) => &states[..*len as usize],
            Self::Boxed(states) => &states[..],
        }
    }
}

impl DerefMut for PollVec {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            PollVec::Inline(len, states) => &mut states[..*len as usize],
            Self::Boxed(states) => &mut states[..],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{PollVec, MAX_INLINE_ENTRIES};

    #[test]
    fn type_size() {
        assert_eq!(
            std::mem::size_of::<PollVec>(),
            std::mem::size_of::<usize>() * 3
        );
    }

    #[test]
    fn boxed_does_not_allocate_twice() {
        // Make sure the debug_assertions in PollStates::new() don't fail.
        let _ = PollVec::new(MAX_INLINE_ENTRIES + 10);
    }
}
