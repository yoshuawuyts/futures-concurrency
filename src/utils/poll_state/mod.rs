/// Enumerate the current poll state.
#[derive(Debug, Clone, Copy, Default)]
#[repr(u8)]
pub(crate) enum PollState {
    /// Polling the underlying future or stream.
    #[default]
    Pending,
    /// Data has been written to the output structure, and is now ready to be
    /// read.
    Ready,
    /// The underlying future or stream has finished yielding data and all data
    /// has been read. We can now stop reasoning about it.
    Consumed,
}
