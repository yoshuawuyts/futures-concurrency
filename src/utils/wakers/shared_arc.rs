use core::task::{RawWaker, RawWakerVTable, Waker};

use alloc::sync::Arc;

pub(super) unsafe trait SharedArcContent {
    /// Get the reference of the redirect slice
    fn get_redirect_slice(&self) -> &[*const Self];
    /// Called when the subfuture at the specified index should be polled
    /// Should call `Readiness::set_ready`
    fn wake_index(&self, index: usize);
}

pub(super) unsafe fn waker_from_redirec_position<A: SharedArcContent>(
    arc: Arc<A>,
    index: usize,
) -> Waker {
    unsafe fn clone_waker<A: SharedArcContent>(pointer: *const ()) -> RawWaker {
        let pointer = pointer as *const *const A;
        unsafe { Arc::increment_strong_count(*pointer) };
        RawWaker::new(pointer as *const (), create_vtable::<A>())
    }

    unsafe fn wake_by_ref<A: SharedArcContent>(pointer: *const ()) {
        let pointer = pointer as *const *const A;
        let raw: *const A = unsafe { *pointer };
        let arc_content: &A = unsafe { &*raw };
        let slice = arc_content.get_redirect_slice().as_ptr();
        let index = unsafe { pointer.offset_from(slice) } as usize;
        arc_content.wake_index(index);
    }

    unsafe fn drop_waker<A: SharedArcContent>(pointer: *const ()) {
        let pointer = pointer as *const *const A;
        unsafe { Arc::decrement_strong_count(*pointer) };
    }

    unsafe fn wake<A: SharedArcContent>(pointer: *const ()) {
        unsafe {
            wake_by_ref::<A>(pointer);
            drop_waker::<A>(pointer);
        }
    }

    fn create_vtable<A: SharedArcContent>() -> &'static RawWakerVTable {
        &RawWakerVTable::new(
            clone_waker::<A>,
            wake::<A>,
            wake_by_ref::<A>,
            drop_waker::<A>,
        )
    }

    let redirect = arc.get_redirect_slice();
    let pointer = unsafe { redirect.as_ptr().add(index) } as *const ();
    core::mem::forget(arc);

    unsafe { Waker::from_raw(RawWaker::new(pointer, create_vtable::<A>())) }
}
