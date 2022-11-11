/// Enumerate the current poll state.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum PollState {
    /// Polling the underlying future.
    #[default]
    Pending = 0,
    /// Data has been written to the output structure
    /// and the future should no longer be polled.
    Done = 1,
    /// Data has been consumed from the output structure,
    /// and we should no longer reason about it.
    Consumed = 2,

    // LLVM seems to like it of there are 4 entries here.
    // That seems to reliably enable optimizations for
    // PollState::from_bits() when called with `num & 0b11`.
    //
    // This can be used for the future not started yet state.
    Dummy = 3,
}

impl PollState {
    // Default::default() does not work with const?
    const DEFAULT_WORD: EntriesWord = PollState::Pending.to_word();

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

    const fn to_bits(self) -> u8 {
        let bits = self as u8;
        debug_assert!(bits <= 3);
        bits
    }

    const fn from_bits(bits: u8) -> Self {
        let val = match bits {
            0 => Self::Pending,
            1 => Self::Done,
            2 => Self::Consumed,
            3 => Self::Dummy,
            _ => unreachable!(),
        };

        debug_assert!(val as u8 == bits);

        val
    }

    const fn to_word(self) -> EntriesWord {
        const fn set_entry(word: EntriesWord, index: usize, bits: u8) -> EntriesWord {
            if index >= ENTRIES_PER_WORD {
                word
            } else {
                set_entry(
                    word | ((bits as EntriesWord) << (index * 2)),
                    index + 1,
                    bits,
                )
            }
        }
        set_entry(0, 0, self.to_bits())
    }
}

/// The max number of entries `PollStates` can store without heap allocations.
const MAX_INLINE_ENTRY_COUNT: usize = ENTRIES_PER_WORD;
const ENTRIES_PER_WORD: usize = (std::mem::size_of::<EntriesWord>() * 8) / 2;

type EntriesWord = u64;

pub(crate) struct PollStates {
    len: usize,
    entries: PollStateEntries,
}

enum PollStateEntries {
    Inline(EntriesWord),
    Boxed(Vec<EntriesWord>),
}

impl PollStates {
    #[inline]
    pub(crate) fn get(&self, index: usize) -> PollState {
        debug_assert!(index < self.len);
        match &self.entries {
            PollStateEntries::Inline(entries_word) => get_state_in_word(*entries_word, index),
            PollStateEntries::Boxed(states) => get_state(states, index),
        }
    }

    #[inline]
    pub(crate) fn set(&mut self, index: usize, state: PollState) {
        debug_assert!(index < self.len);
        match &mut self.entries {
            PollStateEntries::Inline(entries_word) => {
                *entries_word = set_state_in_word(*entries_word, index, state)
            }
            PollStateEntries::Boxed(states) => set_state(states, index, state),
        }
    }

    #[inline]
    pub(crate) fn set_all(&mut self, state: PollState) {
        let word = match state {
            PollState::Pending => PollState::Pending.to_word(),
            PollState::Done => PollState::Done.to_word(),
            PollState::Consumed => PollState::Consumed.to_word(),
            PollState::Dummy => PollState::Dummy.to_word(),
        };

        match &mut self.entries {
            PollStateEntries::Inline(entries_word) => *entries_word = word,
            PollStateEntries::Boxed(states) => states.fill(word),
        }
    }

    #[inline]
    pub(crate) fn len(&self) -> usize {
        self.len
    }

    #[inline]
    pub(crate) fn new(len: usize) -> Self {
        if len <= MAX_INLINE_ENTRY_COUNT {
            Self {
                len,
                entries: PollStateEntries::Inline(PollState::DEFAULT_WORD),
            }
        } else {
            let mut words = Vec::new();
            words.resize(words_needed_for_entry_count(len), PollState::DEFAULT_WORD);

            Self {
                len,
                entries: PollStateEntries::Boxed(words),
            }
        }
    }

    pub(crate) fn for_each(&self, mut f: impl FnMut(usize, PollState)) {
        match &self.entries {
            PollStateEntries::Inline(entries_word) => {
                for_each_state_in_word(*entries_word, self.len, 0, f);
            }
            PollStateEntries::Boxed(states) => {
                let full_word_count = self.len / ENTRIES_PER_WORD;
                let full_words = &states[..full_word_count];

                for (word_index, word) in full_words.iter().enumerate() {
                    let entry_index = word_index * ENTRIES_PER_WORD;
                    for_each_state_in_word(*word, ENTRIES_PER_WORD, entry_index, &mut f);
                }

                let rest = self.len - full_word_count * ENTRIES_PER_WORD;

                if rest > 0 {
                    for_each_state_in_word(
                        states.last().cloned().unwrap(),
                        rest,
                        full_word_count * ENTRIES_PER_WORD,
                        f,
                    );
                }
            }
        }
    }

    pub(crate) fn to_vec(&self) -> Vec<PollState> {
        let mut items = Vec::with_capacity(self.len);
        self.for_each(|_, state| items.push(state));
        items
    }
}

#[inline]
fn get_state(words: &[EntriesWord], index: usize) -> PollState {
    let (array_index, shift) = index_and_shift(index);
    let bits = (words[array_index] >> shift) as u8 & 0b11;
    PollState::from_bits(bits)
}

#[inline]
fn for_each_state_in_word(
    mut entries_word: EntriesWord,
    entry_count: usize,
    index_offset: usize,
    mut f: impl FnMut(usize, PollState),
) {
    for local_index in 0..entry_count {
        f(
            local_index + index_offset,
            PollState::from_bits((entries_word as u8) & 0b11),
        );
        entries_word >>= 2;
    }
}

#[inline]
fn get_state_in_word(entries_word: EntriesWord, local_index: usize) -> PollState {
    let bits = (entries_word >> (local_index * 2)) & 0b11;
    PollState::from_bits(bits as u8)
}

#[inline]
fn set_state_in_word(
    entries_word: EntriesWord,
    local_index: usize,
    state: PollState,
) -> EntriesWord {
    let shift = local_index * 2;
    let mask = !(0b11 << shift);
    let bits = state.to_bits() as u64;
    (entries_word & mask) | (bits << shift)
}

#[inline]
fn set_state(bytes: &mut [EntriesWord], index: usize, state: PollState) {
    let (array_index, shift) = index_and_shift(index);
    let bits = (state.to_bits() as EntriesWord) << shift;
    bytes[array_index] = (bytes[array_index] & !(0b11 << shift)) | bits;
}

#[inline]
const fn words_needed_for_entry_count(entry_count: usize) -> usize {
    (entry_count + ENTRIES_PER_WORD - 1) / ENTRIES_PER_WORD
}

#[inline]
fn index_and_shift(index: usize) -> (usize, usize) {
    let array_index = index / ENTRIES_PER_WORD;
    let bit_shift = (index & (ENTRIES_PER_WORD - 1)) * 2;
    (array_index, bit_shift)
}

#[cfg(test)]
mod tests {
    use crate::utils::{poll_state::ENTRIES_PER_WORD, random};

    use super::{index_and_shift, PollState, PollStateEntries, PollStates};

    const STATES: [PollState; 3] = [PollState::Consumed, PollState::Pending, PollState::Done];

    fn to_vec(states: &PollStates) -> Vec<PollState> {
        let mut v = vec![];
        states.for_each(|_, state| v.push(state));
        v
    }

    #[test]
    fn indices() {
        assert_eq!(index_and_shift(0), (0, 0));
        assert_eq!(index_and_shift(1), (0, 2));
        assert_eq!(index_and_shift(2), (0, 4));
        assert_eq!(index_and_shift(3), (0, 6));
        assert_eq!(
            index_and_shift(ENTRIES_PER_WORD - 1),
            (0, (ENTRIES_PER_WORD - 1) * 2)
        );
        assert_eq!(index_and_shift(ENTRIES_PER_WORD), (1, 0));
        assert_eq!(index_and_shift(ENTRIES_PER_WORD + 1), (1, 2));
    }

    #[test]
    fn get_set_random() {
        use super::*;

        const LENGTHS: &[usize] = &[
            0, 1, 2, 3, 4, 5, 8, 15, 16, 17, 21, 22, 23, 31, 32, 33, 63, 64, 65, 500, 10000,
        ];

        for &len in LENGTHS {
            let mut states = PollStates::new(len);

            assert_eq!(states.len(), len);
            let mut reference = vec![PollState::default(); len];

            assert_eq!(reference, states.to_vec());

            let op_count = if cfg!(miri) { 20 } else { 1000 };

            if len > 0 {
                for _ in 0..op_count {
                    let index = random(len as u32) as usize;
                    let state = STATES[random(STATES.len() as u32) as usize];

                    reference[index] = state;
                    states.set(index, state);

                    assert_eq!(reference, states.to_vec());
                }
            }
        }
    }
}
