use std::{fmt::Debug, mem::ManuallyDrop, ops::ControlFlow, ptr::NonNull};

use crate::collector::{Collector, CollectorBase};

///
pub struct BoxedCollector<T, O> {
    data: NonNull<()>,
    // Can't use `&'static` because the compile will unreasonably demand
    // us to put `T: 'static`.
    vtable: NonNull<VTable<T, O>>,
}

struct VTable<T, O> {
    finish: unsafe fn(NonNull<()>) -> O,
    break_hint: unsafe fn(NonNull<()>) -> ControlFlow<()>,
    collect: unsafe fn(NonNull<()>, T) -> ControlFlow<()>,
    drop: unsafe fn(NonNull<()>),
}

impl<T, O> BoxedCollector<T, O> {
    ///
    pub fn new<C>(collector: C) -> Self
    where
        C: Collector<T, Output = O>,
    {
        Self {
            data: NonNull::from(Box::leak(Box::new(collector))).cast(),
            // Added a const block to assert that everything in this block
            // is const-evaluated, which is needed for static promotion.
            vtable: const {
                NonNull::from_ref(&VTable {
                    finish: |data| unsafe {
                        let collector = Box::from_raw(data.cast::<C>().as_ptr());
                        collector.finish()
                    },
                    break_hint: |data| unsafe {
                        let collector = data.cast::<C>().as_ref();
                        collector.break_hint()
                    },
                    collect: |data, item| unsafe {
                        let collector = data.cast::<C>().as_mut();
                        collector.collect(item)
                    },
                    drop: |data| unsafe { drop(Box::from_raw(data.cast::<C>().as_ptr())) },
                })
            },
        }
    }
}

impl<T, O> VTable<T, O> {}

impl<T, O> Drop for BoxedCollector<T, O> {
    fn drop(&mut self) {
        unsafe { (self.vtable.as_ref().drop)(self.data) }
    }
}

impl<T, O> CollectorBase for BoxedCollector<T, O> {
    type Output = O;

    fn finish(self) -> Self::Output {
        // We don't want to drop ourselves. `finish()` will drop for us.
        let this = ManuallyDrop::new(self);
        unsafe { (this.vtable.as_ref().finish)(this.data) }
    }
}

impl<T, O> Debug for BoxedCollector<T, O> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BoxedCollector").finish()
    }
}

#[cfg(feature = "std")]
mod _experiment {
    use std::ptr::NonNull;

    pub struct _Bar<A, T> {
        pub _x: A,
        pub _y: fn(T) -> T,
    }

    pub const fn _bar<T>() -> NonNull<_Bar<i32, T>> {
        NonNull::from_ref(&_Bar { _x: 1, _y: |x| x })
    }

    pub fn _use_bar<'a>() -> NonNull<_Bar<i32, Vec<&'a str>>> {
        _bar()
    }

    // Compile error
    // pub const fn _bar2<T>() -> NonNull<_Bar<String, T>> {
    //     NonNull::from_ref(&_Bar {
    //         _x: String::from("noble and singer"),
    //         _y: |x| x,
    //     })
    // }
}
