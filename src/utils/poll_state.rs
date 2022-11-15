use std::ops::{Deref, DerefMut};

/// Enumerate the current poll state.
#[derive(Debug, Clone, Copy, Default)]
#[repr(u8)]
pub(crate) enum PollState {
    /// Polling the underlying future.
    #[default]
    Pending,
    /// Data has been written to the output structure
    /// and the future should no longer be polled.
    Done,
    /// Data has been consumed from the output structure,
    /// and we should no longer reason about it.
    Consumed,
}

impl PollState {
    /// Returns `true` if the metadata is [`Pending`].
    ///
    /// [`Pending`]: Metadata::Pending
    #[must_use]
    pub(crate) fn is_pending(&self) -> bool {
        matches!(self, Self::Pending)
    }

    /// Returns `true` if the poll state is [`Done`].
    ///
    /// [`Done`]: PollState::Done
    #[must_use]
    pub(crate) fn is_done(&self) -> bool {
        matches!(self, Self::Done)
    }

    /// Returns `true` if the poll state is [`Consumed`].
    ///
    /// [`Consumed`]: PollState::Consumed
    #[must_use]
    pub(crate) fn is_consumed(&self) -> bool {
        matches!(self, Self::Consumed)
    }
}

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

pub(crate) enum PollStates {
    Inline(u8, [PollState; MAX_INLINE_ENTRIES]),
    Boxed(Box<[PollState]>),
}

impl core::fmt::Debug for PollStates {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Inline(len, states) => f
                // .debug_tuple("Inline")
                .debug_list()
                .entries(&states[..(*len as usize)])
                .finish(),
            Self::Boxed(states) => f.debug_list().entries(&**states).finish(),
        }
    }
}

impl PollStates {
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
}

impl Deref for PollStates {
    type Target = [PollState];

    fn deref(&self) -> &Self::Target {
        match self {
            PollStates::Inline(len, states) => &states[..*len as usize],
            Self::Boxed(states) => &states[..],
        }
    }
}

impl DerefMut for PollStates {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            PollStates::Inline(len, states) => &mut states[..*len as usize],
            Self::Boxed(states) => &mut states[..],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{PollStates, MAX_INLINE_ENTRIES};

    #[test]
    fn type_size() {
        assert_eq!(
            std::mem::size_of::<PollStates>(),
            std::mem::size_of::<usize>() * 3
        );
    }

    #[test]
    fn boxed_does_not_allocate_twice() {
        // Make sure the debug_assertions in PollStates::new() don't fail.
        let _ = PollStates::new(MAX_INLINE_ENTRIES + 10);
    }
}
