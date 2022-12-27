use std::{
    sync::Arc,
    task::{RawWaker, RawWakerVTable, Waker},
};

pub(super) trait WakerArrayTrait {
    fn get_wake_data_slice(&self) -> &[*const Self];
    fn wake_index(&self, index: usize);
}

pub(super) unsafe fn waker_from_position<A: WakerArrayTrait>(pointer: *const *const A) -> Waker {
    unsafe fn clone_waker<A: WakerArrayTrait>(pointer: *const ()) -> RawWaker {
        let pointer = pointer as *const *const A;
        let raw = *pointer; // This is the raw pointer of Arc<Inner>.

        // We're creating a new Waker, so we need to increment the count.
        Arc::increment_strong_count(raw);

        RawWaker::new(pointer as *const (), create_vtable::<A>())
    }

    // Convert a pointer to a wake_data slot to the Arc<Inner>.
    unsafe fn to_arc<A: WakerArrayTrait>(pointer: *const *const A) -> Arc<A> {
        let raw = *pointer;
        Arc::from_raw(raw)
    }
    unsafe fn wake<A: WakerArrayTrait, const BY_REF: bool>(pointer: *const ()) {
        let pointer = pointer as *const *const A;
        let arc = to_arc::<A>(pointer);
        // Calculate the index
        let index = ((pointer as usize) // This is the slot our pointer points to.
            - (arc.get_wake_data_slice() as *const [*const A] as *const () as usize)) // This is the starting address of wake_data.
            / std::mem::size_of::<*const A>();

        arc.wake_index(index);

        // Dropping the Arc would decrement the strong count.
        // We only want to do that when we're not waking by ref.
        if BY_REF {
            std::mem::forget(arc);
        } else {
            std::mem::drop(arc);
        }
    }
    unsafe fn drop_waker<A: WakerArrayTrait>(pointer: *const ()) {
        let pointer = pointer as *const *const A;
        let arc = to_arc::<A>(pointer);
        // Decrement the strong count by dropping the Arc.
        std::mem::drop(arc);
    }
    fn create_vtable<A: WakerArrayTrait>() -> &'static RawWakerVTable {
        &RawWakerVTable::new(
            clone_waker::<A>,
            wake::<A, false>,
            wake::<A, true>,
            drop_waker::<A>,
        )
    }
    Waker::from_raw(clone_waker::<A>(pointer as *const ()))
}
