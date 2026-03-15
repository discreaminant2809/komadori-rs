//! [`Collector`]s for [`Sender`] and [`SyncSender`].
//!
//! This module corresponds to [`std::sync::mpsc`].
//!
//! [`Collector`]: crate::collector::Collector

use std::{
    ops::ControlFlow,
    sync::mpsc::{Sender, SyncSender},
};

use crate::collector::CollectorBase;

/// A collector that sends items through a [`std::sync::mpsc::channel()`].
/// Its [`Output`](CollectorBase::Output) is [`Sender`].
///
/// If the receiver has hung up, this collector returns [`Break(())`](ControlFlow::Break).
///
/// Unlike [`send`](Sender::send), items collected after the
/// receiver has hung up are simply lost. They cannot be recovered.
///
/// This struct is created by `Sender::into_collector()`.
///
/// # Examples
///
/// ```
/// use std::{thread, sync::{mpsc, Mutex, Condvar}};
/// use komadori::prelude::*;
///
/// let (tx, rx) = mpsc::channel();
/// let hung = Mutex::new(false);
/// let notifier = Condvar::new();
///
/// thread::scope(|s| {
///     let handle = s.spawn(|| {
///         let mut tx = tx.into_collector();
///
///         assert!(tx.collect_many([1, 2, 3]).is_continue());
///
///         // Wait until the receiver hangs.
///         notifier.wait_while(
///             hung.lock().unwrap(),
///             |hung| !*hung,
///         );
///
///         assert!(tx.collect(4).is_break());
///     });
///
///     assert_eq!(rx.recv(), Ok(1));
///     assert_eq!(rx.recv(), Ok(2));
///     assert_eq!(rx.recv(), Ok(3));
///     
///     drop(rx);
///     *hung.lock().unwrap() = true;
///     notifier.notify_one();
///     assert!(handle.join().is_ok());
/// });
/// ```
///
/// [`Collector`]: crate::collector::Collector
pub struct IntoCollector<T>(Sender<T>);

/// A collector that sends items through a [`std::sync::mpsc::channel()`].
/// Its [`Output`](CollectorBase::Output) is [`&Sender`](Sender).
///
/// If the receiver has hung up, this collector returns [`Break(())`](ControlFlow::Break).
///
/// Unlike [`send`](Sender::send), items collected after the
/// receiver has hung up are simply lost. They cannot be recovered.
///
/// This struct is created by `Sender::collector()`.
///
/// # Examples
///
/// ```
/// use std::{thread, sync::{mpsc, Mutex, Condvar}};
/// use komadori::prelude::*;
///
/// let (tx, rx) = mpsc::channel();
/// let hung = Mutex::new(false);
/// let notifier = Condvar::new();
///
/// thread::scope(|s| {
///     let handle = s.spawn(|| {
///         let mut tx = tx.collector();
///
///         assert!(tx.collect_many([1, 2, 3]).is_continue());
///
///         // Wait until the receiver hangs.
///         notifier.wait_while(
///             hung.lock().unwrap(),
///             |hung| !*hung,
///         );
///
///         assert!(tx.collect(4).is_break());
///     });
///
///     assert_eq!(rx.recv(), Ok(1));
///     assert_eq!(rx.recv(), Ok(2));
///     assert_eq!(rx.recv(), Ok(3));
///     
///     drop(rx);
///     *hung.lock().unwrap() = true;
///     notifier.notify_one();
///     assert!(handle.join().is_ok());
/// });
/// ```
///
/// [`Collector`]: crate::collector::Collector
pub struct Collector<'a, T>(&'a Sender<T>);

/// A collector that sends items through a [`std::sync::mpsc::sync_channel()`].
/// Its [`Output`](CollectorBase::Output) is [`SyncSender`].
///
/// If the receiver has hung up, this collector returns [`Break(())`](ControlFlow::Break).
///
/// Unlike [`send`](SyncSender::send), items collected after the
/// receiver has hung up are simply lost. They cannot be recovered.
///
/// This struct is created by `SyncSender::into_collector()`.
///
/// # Examples
///
/// ```
/// use std::{thread, sync::{mpsc, Mutex, Condvar}};
/// use komadori::prelude::*;
///
/// let (tx, rx) = mpsc::sync_channel(1);
/// let hung = Mutex::new(false);
/// let notifier = Condvar::new();
///
/// thread::scope(|s| {
///     let handle = s.spawn(|| {
///         let mut tx = tx.into_collector();
///
///         assert!(tx.collect_many([1, 2, 3]).is_continue());
///
///         // Wait until the receiver hangs.
///         notifier.wait_while(
///             hung.lock().unwrap(),
///             |hung| !*hung,
///         );
///
///         assert!(tx.collect(4).is_break());
///     });
///
///     assert_eq!(rx.recv(), Ok(1));
///     assert_eq!(rx.recv(), Ok(2));
///     assert_eq!(rx.recv(), Ok(3));
///     
///     drop(rx);
///     *hung.lock().unwrap() = true;
///     notifier.notify_one();
///     assert!(handle.join().is_ok());
/// });
/// ```
///
/// [`Collector`]: crate::collector::Collector
pub struct IntoSyncCollector<T>(SyncSender<T>);

/// A collector that sends items through a [`std::sync::mpsc::sync_channel()`].
/// Its [`Output`](CollectorBase::Output) is [`&SyncSender`](SyncSender).
///
/// If the receiver has hung up, this collector returns [`Break(())`](ControlFlow::Break).
///
/// Unlike [`send`](SyncSender::send), items collected after the
/// receiver has hung up are simply lost. They cannot be recovered.
///
/// This struct is created by `SyncSender::collector()`.
///
/// # Examples
///
/// ```
/// use std::{thread, sync::{mpsc, Mutex, Condvar}};
/// use komadori::prelude::*;
///
/// let (tx, rx) = mpsc::sync_channel(1);
/// let hung = Mutex::new(false);
/// let notifier = Condvar::new();
///
/// thread::scope(|s| {
///     let handle = s.spawn(|| {
///         let mut tx = tx.collector();
///
///         assert!(tx.collect_many([1, 2, 3]).is_continue());
///
///         // Wait until the receiver hangs.
///         notifier.wait_while(
///             hung.lock().unwrap(),
///             |hung| !*hung,
///         );
///
///         assert!(tx.collect(4).is_break());
///     });
///
///     assert_eq!(rx.recv(), Ok(1));
///     assert_eq!(rx.recv(), Ok(2));
///     assert_eq!(rx.recv(), Ok(3));
///     
///     drop(rx);
///     *hung.lock().unwrap() = true;
///     notifier.notify_one();
///     assert!(handle.join().is_ok());
/// });
/// ```
///
/// [`Collector`]: crate::collector::Collector
pub struct SyncCollector<'a, T>(&'a SyncSender<T>);

impl<T> crate::collector::IntoCollectorBase for Sender<T> {
    type Output = Self;

    type IntoCollector = IntoCollector<T>;

    #[inline]
    fn into_collector(self) -> Self::IntoCollector {
        IntoCollector(self)
    }
}

impl<T> CollectorBase for IntoCollector<T> {
    type Output = Sender<T>;

    #[inline]
    fn finish(self) -> Self::Output {
        self.0
    }
}

impl<T> crate::collector::Collector<T> for IntoCollector<T> {
    #[inline]
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        match self.0.send(item) {
            Ok(_) => ControlFlow::Continue(()),
            Err(_) => ControlFlow::Break(()),
        }
    }

    // The default implementations for other methods are sufficient.
}

impl<'a, T> crate::collector::IntoCollectorBase for &'a Sender<T> {
    type Output = Self;

    type IntoCollector = Collector<'a, T>;

    #[inline]
    fn into_collector(self) -> Self::IntoCollector {
        Collector(self)
    }
}

impl<'a, T> CollectorBase for Collector<'a, T> {
    type Output = &'a Sender<T>;

    #[inline]
    fn finish(self) -> Self::Output {
        self.0
    }
}

impl<'a, T> crate::collector::Collector<T> for Collector<'a, T> {
    #[inline]
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        match self.0.send(item) {
            Ok(_) => ControlFlow::Continue(()),
            Err(_) => ControlFlow::Break(()),
        }
    }

    // The default implementations for other methods are sufficient.
}

impl<T> crate::collector::IntoCollectorBase for SyncSender<T> {
    type Output = Self;

    type IntoCollector = IntoSyncCollector<T>;

    #[inline]
    fn into_collector(self) -> Self::IntoCollector {
        IntoSyncCollector(self)
    }
}

impl<T> CollectorBase for IntoSyncCollector<T> {
    type Output = SyncSender<T>;

    #[inline]
    fn finish(self) -> Self::Output {
        self.0
    }
}

impl<T> crate::collector::Collector<T> for IntoSyncCollector<T> {
    #[inline]
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        match self.0.send(item) {
            Ok(_) => ControlFlow::Continue(()),
            Err(_) => ControlFlow::Break(()),
        }
    }

    // The default implementations for other methods are sufficient.
}

impl<'a, T> crate::collector::IntoCollectorBase for &'a SyncSender<T> {
    type Output = Self;

    type IntoCollector = SyncCollector<'a, T>;

    #[inline]
    fn into_collector(self) -> Self::IntoCollector {
        SyncCollector(self)
    }
}

impl<'a, T> CollectorBase for SyncCollector<'a, T> {
    type Output = &'a SyncSender<T>;

    #[inline]
    fn finish(self) -> Self::Output {
        self.0
    }
}

impl<'a, T> crate::collector::Collector<T> for SyncCollector<'a, T> {
    #[inline]
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        match self.0.send(item) {
            Ok(_) => ControlFlow::Continue(()),
            Err(_) => ControlFlow::Break(()),
        }
    }

    // The default implementations for other methods are sufficient.
}

macro_rules! debug_clone_impl {
    ($ty_name:ident<$($lts:lifetime,)* $($generics:ident),*>) => {
        impl<T> std::fmt::Debug for $ty_name<$($lts,)* $($generics),*> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_tuple(stringify!($ty_name))
                    .field(&self.0)
                    .finish()
            }
        }

        impl<T> Clone for $ty_name<$($lts,)* $($generics),*> {
            fn clone(&self) -> Self {
                Self(Clone::clone(&self.0))
            }
        }
    };
}

debug_clone_impl!(Collector<'_, T>);
debug_clone_impl!(SyncCollector<'_, T>);
debug_clone_impl!(IntoCollector<T>);
debug_clone_impl!(IntoSyncCollector<T>);
