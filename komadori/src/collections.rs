//! [`Collector`]s for collections in the standard library
//!
//! This module corresponds to [`std::collections`].

pub mod binary_heap;
pub mod btree_map;
pub mod btree_set;
#[cfg(feature = "std")]
// So that doc.rs doesn't put both "std" and "alloc" in feature flag.
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
pub mod hash_map;
#[cfg(feature = "std")]
// So that doc.rs doesn't put both "std" and "alloc" in feature flag.
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
pub mod hash_set;
pub mod linked_list;
pub mod vec_deque;

use std::ops::ControlFlow;

use crate::collector::{Collector, CollectorBase, IntoCollectorBase};

#[cfg(feature = "std")]
use std::{
    cmp::Eq,
    collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap, HashSet, LinkedList, VecDeque},
    hash::{BuildHasher, Hash},
};

#[cfg(not(feature = "std"))]
// Hashtables are not in `alloc`.
use alloc::collections::{BTreeMap, BTreeSet, BinaryHeap, LinkedList, VecDeque};

#[cfg(feature = "alloc")]
use std::cmp::Ord;

macro_rules! collector_impl {
    (
        $feature:literal, $mod:ident::$coll_name:ident<$($generic:ident),*>, $item_ty:ty,
        $item_pat:pat_param, $push_method_name:ident($($item_args:expr),*)
        $(, $gen_bound:ident: $bound:path)* $(,)?
    ) => {
        #[cfg(feature = $feature)]
        // So that doc.rs doesn't put both "std" and "alloc" in feature flag.
        #[cfg_attr(docsrs, doc(cfg(feature = $feature)))]
        impl<$($generic),*> IntoCollectorBase for $coll_name<$($generic),*>
        where
            $($gen_bound: $bound,)*
        {
            type Output = Self;
            type IntoCollector = $mod::IntoCollector<$($generic),*>;

            #[inline]
            fn into_collector(self) -> Self::IntoCollector {
                $mod::IntoCollector(self)
            }
        }

        #[cfg(feature = $feature)]
        // So that doc.rs doesn't put both "std" and "alloc" in feature flag.
        #[cfg_attr(docsrs, doc(cfg(feature = $feature)))]
        impl<'a, $($generic),*> IntoCollectorBase for &'a mut $coll_name<$($generic),*>
        where
            $($gen_bound: $bound,)*
        {
            type Output = Self;
            type IntoCollector = $mod::CollectorMut<'a, $($generic),*>;

            #[inline]
            fn into_collector(self) -> Self::IntoCollector {
                $mod::CollectorMut(self)
            }
        }

        #[cfg(feature = $feature)]
        // So that doc.rs doesn't put both "std" and "alloc" in feature flag.
        #[cfg_attr(docsrs, doc(cfg(feature = $feature)))]
        impl<$($generic),*> CollectorBase for $mod::IntoCollector<$($generic),*> {
            type Output = $coll_name<$($generic),*>;

            #[inline]
            fn finish(self) -> Self::Output {
                self.0
            }
        }

        #[cfg(feature = $feature)]
        // So that doc.rs doesn't put both "std" and "alloc" in feature flag.
        #[cfg_attr(docsrs, doc(cfg(feature = $feature)))]
        impl<$($generic),*> Collector<$item_ty> for $mod::IntoCollector<$($generic),*>
        where
            $($gen_bound: $bound,)*
        {
            #[inline]
            fn collect(&mut self, $item_pat: $item_ty) -> ControlFlow<()> {
                // It returns a `bool`, so we will return a `ControlFlow` based on it, right?
                // No. `false` is just a signal that "it cannot collect the item at the moment,"
                // not "it cannot collect items from now on."
                self.0.$push_method_name($($item_args),*);
                ControlFlow::Continue(())
            }

            #[inline]
            fn collect_many(&mut self, items: impl IntoIterator<Item = $item_ty>) -> ControlFlow<()> {
                self.0.extend(items);
                ControlFlow::Continue(())
            }

            #[inline]
            fn collect_then_finish(mut self, items: impl IntoIterator<Item = $item_ty>) -> Self::Output {
                self.0.extend(items);
                self.0
            }
        }

        // #[cfg(feature = $feature)]
        // // So that doc.rs doesn't put both "std" and "alloc" in feature flag.
        // #[cfg_attr(docsrs, doc(cfg(feature = $feature)))]
        // impl<'i, $($generic),*> Collector<&'i $item_ty> for $mod::IntoCollector<$($generic),*>
        // where
        //     $($gen_bound: $bound,)*
        //     $($copy_bound_left: $copy_bound_right),*
        // {
        //     #[inline]
        //     fn collect(&mut self, &$item_pat: &$item_ty) -> ControlFlow<()> {
        //         // It returns a `bool`, so we will return a `ControlFlow` based on it, right?
        //         // No. `false` is just a signal that "it cannot collect the item at the moment,"
        //         // not "it cannot collect items from now on."
        //         self.0.$push_method_name($($item_args),*);
        //         ControlFlow::Continue(())
        //     }

        //     #[inline]
        //     fn collect_many(&mut self, items: impl IntoIterator<Item = &'i $item_ty>) -> ControlFlow<()> {
        //         self.0.extend(items.into_iter().map(|&item| item));
        //         ControlFlow::Continue(())
        //     }

        //     #[inline]
        //     fn collect_then_finish(mut self, items: impl IntoIterator<Item = &'i $item_ty>) -> Self::Output {
        //         self.0.extend(items.into_iter().map(|&item| item));
        //         self.0
        //     }
        // }

        // #[cfg(feature = $feature)]
        // // So that doc.rs doesn't put both "std" and "alloc" in feature flag.
        // #[cfg_attr(docsrs, doc(cfg(feature = $feature)))]
        // impl<'i, $($generic),*> Collector<&'i mut $item_ty> for $mod::IntoCollector<$($generic),*>
        // where
        //     $($gen_bound: $bound,)*
        //     $($copy_bound_left: $copy_bound_right),*
        // {
        //     #[inline]
        //     fn collect(&mut self, &mut $item_pat: &mut $item_ty) -> ControlFlow<()> {
        //         // It returns a `bool`, so we will return a `ControlFlow` based on it, right?
        //         // No. `false` is just a signal that "it cannot collect the item at the moment,"
        //         // not "it cannot collect items from now on."
        //         self.0.$push_method_name($($item_args),*);
        //         ControlFlow::Continue(())
        //     }

        //     #[inline]
        //     fn collect_many(&mut self, items: impl IntoIterator<Item = &'i mut $item_ty>) -> ControlFlow<()> {
        //         self.0.extend(items.into_iter().map(|&mut item| item));
        //         ControlFlow::Continue(())
        //     }

        //     #[inline]
        //     fn collect_then_finish(mut self, items: impl IntoIterator<Item = &'i mut $item_ty>) -> Self::Output {
        //         self.0.extend(items.into_iter().map(|&mut item| item));
        //         self.0
        //     }
        // }

        #[cfg(feature = $feature)]
        // So that doc.rs doesn't put both "std" and "alloc" in feature flag.
        #[cfg_attr(docsrs, doc(cfg(feature = $feature)))]
        impl<'a, $($generic),*> CollectorBase for $mod::CollectorMut<'a, $($generic),*> {
            type Output = &'a mut $coll_name<$($generic),*>;

            #[inline]
            fn finish(self) -> Self::Output {
                self.0
            }
        }

        #[cfg(feature = $feature)]
        // So that doc.rs doesn't put both "std" and "alloc" in feature flag.
        #[cfg_attr(docsrs, doc(cfg(feature = $feature)))]
        impl<'a, $($generic),*> Collector<$item_ty> for $mod::CollectorMut<'a, $($generic),*>
        where
            $($gen_bound: $bound,)*
        {
            #[inline]
            fn collect(&mut self, $item_pat: $item_ty) -> ControlFlow<()> {
                // It returns a `bool`, so we will return a `ControlFlow` based on it, right?
                // No. `false` is just a signal that "it cannot collect the item at the moment,"
                // not "it cannot collect items from now on."
                self.0.$push_method_name($($item_args),*);
                ControlFlow::Continue(())
            }

            #[inline]
            fn collect_many(&mut self, items: impl IntoIterator<Item = $item_ty>) -> ControlFlow<()> {
                self.0.extend(items);
                ControlFlow::Continue(())
            }

            #[inline]
            fn collect_then_finish(self, items: impl IntoIterator<Item = $item_ty>) -> Self::Output {
                self.0.extend(items);
                self.0
            }
        }

        // #[cfg(feature = $feature)]
        // // So that doc.rs doesn't put both "std" and "alloc" in feature flag.
        // #[cfg_attr(docsrs, doc(cfg(feature = $feature)))]
        // impl<'a, 'i, $($generic),*> Collector<&'i $item_ty> for $mod::CollectorMut<'a, $($generic),*>
        // where
        //     $($gen_bound: $bound,)*
        //     $($copy_bound_left: $copy_bound_right),*
        // {
        //     #[inline]
        //     fn collect(&mut self, &$item_pat: &$item_ty) -> ControlFlow<()> {
        //         // It returns a `bool`, so we will return a `ControlFlow` based on it, right?
        //         // No. `false` is just a signal that "it cannot collect the item at the moment,"
        //         // not "it cannot collect items from now on."
        //         self.0.$push_method_name($($item_args),*);
        //         ControlFlow::Continue(())
        //     }

        //     #[inline]
        //     fn collect_many(&mut self, items: impl IntoIterator<Item = &'i $item_ty>) -> ControlFlow<()> {
        //         self.0.extend(items.into_iter().map(|&item| item));
        //         ControlFlow::Continue(())
        //     }

        //     #[inline]
        //     fn collect_then_finish(self, items: impl IntoIterator<Item = &'i $item_ty>) -> Self::Output {
        //         self.0.extend(items.into_iter().map(|&item| item));
        //         self.0
        //     }
        // }

        // #[cfg(feature = $feature)]
        // // So that doc.rs doesn't put both "std" and "alloc" in feature flag.
        // #[cfg_attr(docsrs, doc(cfg(feature = $feature)))]
        // impl<'a, 'i, $($generic),*> Collector<&'i mut $item_ty> for $mod::CollectorMut<'a, $($generic),*>
        // where
        //     $($gen_bound: $bound,)*
        //     $($copy_bound_left: $copy_bound_right),*
        // {
        //     #[inline]
        //     fn collect(&mut self, &mut $item_pat: &mut $item_ty) -> ControlFlow<()> {
        //         // It returns a `bool`, so we will return a `ControlFlow` based on it, right?
        //         // No. `false` is just a signal that "it cannot collect the item at the moment,"
        //         // not "it cannot collect items from now on."
        //         self.0.$push_method_name($($item_args),*);
        //         ControlFlow::Continue(())
        //     }

        //     #[inline]
        //     fn collect_many(&mut self, items: impl IntoIterator<Item = &'i mut $item_ty>) -> ControlFlow<()> {
        //         self.0.extend(items.into_iter().map(|&mut item| item));
        //         ControlFlow::Continue(())
        //     }

        //     #[inline]
        //     fn collect_then_finish(self, items: impl IntoIterator<Item = &'i mut $item_ty>) -> Self::Output {
        //         self.0.extend(items.into_iter().map(|&mut item| item));
        //         self.0
        //     }
        // }

        #[cfg(feature = $feature)]
        // So that doc.rs doesn't put both "std" and "alloc" in feature flag.
        #[cfg_attr(docsrs, doc(cfg(feature = $feature)))]
        impl<$($generic),*> Default for $mod::IntoCollector<$($generic),*>
        where
            $($gen_bound: $bound,)*
            // Needed because of HashMap and HashSet (they also require S: Default).
            $coll_name<$($generic),*>: Default,
        {
            fn default() -> Self {
                // This is to make sure that we can't construct a default value
                // without it being usable right away as a Collector
                // (e.g. BTreeSet<T> missing T: Ord).
                $coll_name::default().into_collector()
            }
        }
    };
}

macro_rules! copy_collector_impl {
    (
        $feature:literal, $mod:ident::$coll_name:ident<$($lt:lifetime),*; $($generic:ident),* $(,)*>, $item_ty:ty,
        $item_pat:pat_param, $push_method_name:ident($($item_args:expr),*)
        $(, $gen_bound:ident: $bound:path)*,
        |$items_param:ident| $transform_items:expr;
    ) => {
        #[cfg(feature = $feature)]
        // So that doc.rs doesn't put both "std" and "alloc" in feature flag.
        #[cfg_attr(docsrs, doc(cfg(feature = $feature)))]
        impl<$($lt,)* $($generic,)*> Collector<$item_ty> for $mod::IntoCollector<$($generic),*>
        where
            $($gen_bound: $bound,)*
        {
            #[inline]
            fn collect(&mut self, $item_pat: $item_ty) -> ControlFlow<()> {
                // It returns a `bool`, so we will return a `ControlFlow` based on it, right?
                // No. `false` is just a signal that "it cannot collect the item at the moment,"
                // not "it cannot collect items from now on."
                self.0.$push_method_name($($item_args),*);
                ControlFlow::Continue(())
            }

            #[inline]
            fn collect_many(&mut self, $items_param: impl IntoIterator<Item = $item_ty>) -> ControlFlow<()> {
                self.0.extend($transform_items);
                ControlFlow::Continue(())
            }

            #[inline]
            fn collect_then_finish(mut self, $items_param: impl IntoIterator<Item = $item_ty>) -> Self::Output {
                self.0.extend($transform_items);
                self.0
            }
        }

        #[cfg(feature = $feature)]
        // So that doc.rs doesn't put both "std" and "alloc" in feature flag.
        #[cfg_attr(docsrs, doc(cfg(feature = $feature)))]
        impl<'a, $($lt,)* $($generic,)*> Collector<$item_ty> for $mod::CollectorMut<'a, $($generic),*>
        where
            $($gen_bound: $bound,)*
        {
            #[inline]
            fn collect(&mut self, $item_pat: $item_ty) -> ControlFlow<()> {
                // It returns a `bool`, so we will return a `ControlFlow` based on it, right?
                // No. `false` is just a signal that "it cannot collect the item at the moment,"
                // not "it cannot collect items from now on."
                self.0.$push_method_name($($item_args),*);
                ControlFlow::Continue(())
            }

            #[inline]
            fn collect_many(&mut self, $items_param: impl IntoIterator<Item = $item_ty>) -> ControlFlow<()> {
                self.0.extend($transform_items);
                ControlFlow::Continue(())
            }

            #[inline]
            fn collect_then_finish(self, $items_param: impl IntoIterator<Item = $item_ty>) -> Self::Output {
                self.0.extend($transform_items);
                self.0
            }
        }
    };
}

collector_impl!(
    "std", hash_map::HashMap<K, V, S>, (K, V),
    (key, value), insert(key, value),
    K: Hash, K: Eq, S: BuildHasher,
);
copy_collector_impl!(
    "std", hash_map::HashMap<'k ,'v; K, V, S>, (&'k K, &'v V),
    (&key, &value), insert(key, value),
    K: Hash, K: Eq, K: Copy, V: Copy, S: BuildHasher,
    |items| items.into_iter().map(|(&k, &v)| (k, v));
);
copy_collector_impl!(
    "std", hash_map::HashMap<'k ,'v; K, V, S>, (&'k mut K, &'v mut V),
    (&mut key, &mut value), insert(key, value),
    K: Hash, K: Eq, K: Copy, V: Copy, S: BuildHasher,
    |items| items.into_iter().map(|(&mut k, &mut v)| (k, v));
);

collector_impl!(
    "std", hash_set::HashSet<T, S>, T,
    item, insert(item),
    T: Hash, T: Eq, S: BuildHasher,
);
copy_collector_impl!(
    "std", hash_set::HashSet<'i; T, S>, &'i T,
    &item, insert(item),
    T: Hash, T: Eq, T: Copy, S: BuildHasher,
    |items| items;
);
copy_collector_impl!(
    "std", hash_set::HashSet<'i; T, S>, &'i mut T,
    &mut item, insert(item),
    T: Hash, T: Eq, T: Copy, S: BuildHasher,
    |items| items.into_iter().map(|&mut item| item);
);

collector_impl!(
    "alloc", btree_map::BTreeMap<K, V>, (K, V),
    (key, value), insert(key, value),
    K: Ord,
);
copy_collector_impl!(
    "alloc", btree_map::BTreeMap<'k, 'v; K, V>, (&'k K, &'v V),
    (&key, &value), insert(key, value),
    K: Ord, K: Copy, V: Copy,
    |items| items.into_iter().map(|(&k, &v)| (k, v));
);
copy_collector_impl!(
    "alloc", btree_map::BTreeMap<'k, 'v; K, V>, (&'k mut K, &'v mut V),
    (&mut key, &mut value), insert(key, value),
    K: Ord, K: Copy, V: Copy,
    |items| items.into_iter().map(|(&mut k, &mut v)| (k, v));
);

collector_impl!(
    "alloc", btree_set::BTreeSet<T>, T,
    item, insert(item),
    T: Ord,
);
copy_collector_impl!(
    "alloc", btree_set::BTreeSet<'i; T>, &'i T,
    &item, insert(item),
    T: Ord, T: Copy,
    |items| items;
);
copy_collector_impl!(
    "alloc", btree_set::BTreeSet<'i; T>, &'i mut T,
    &mut item, insert(item),
    T: Ord, T: Copy,
    |items| items.into_iter().map(|&mut item| item);
);

collector_impl!(
    "alloc", binary_heap::BinaryHeap<T>, T,
    item, push(item),
    T: Ord,
);
copy_collector_impl!(
    "alloc", binary_heap::BinaryHeap<'i; T>, &'i T,
    &item, push(item),
    T: Ord, T: Copy,
    |items| items;
);
copy_collector_impl!(
    "alloc", binary_heap::BinaryHeap<'i; T>, &'i mut T,
    &mut item, push(item),
    T: Ord, T: Copy,
    |items| items.into_iter().map(|&mut item| item);
);

#[rustfmt::skip]
collector_impl!(
    "alloc", linked_list::LinkedList<T>, T,
    item, push_back(item),
);
copy_collector_impl!(
    "alloc", linked_list::LinkedList<'i; T>, &'i T,
    &item, push_back(item),
    T: Copy,
    |items| items;
);
copy_collector_impl!(
    "alloc", linked_list::LinkedList<'i; T>, &'i mut T,
    &mut item, push_back(item),
    T: Copy,
    |items| items.into_iter().map(|&mut item| item);
);

#[rustfmt::skip]
collector_impl!(
    "alloc", vec_deque::VecDeque<T>, T,
    item, push_back(item),
);
copy_collector_impl!(
    "alloc", vec_deque::VecDeque<'i; T>, &'i T,
    &item, push_back(item),
    T: Copy,
    |items| items;
);
copy_collector_impl!(
    "alloc", vec_deque::VecDeque<'i; T>, &'i mut T,
    &mut item, push_back(item),
    T: Copy,
    |items| items.into_iter().map(|&mut item| item);
);
