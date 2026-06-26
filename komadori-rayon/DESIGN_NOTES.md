# Design notes

## Why can't we (yet) unify `*()`, `*_with()`, and `*_init()`?

Because of this compiler bug (apparently):

```text
error[E0277]: the trait bound `Callable<'a, Sender<i32>, {closure@lib.rs:433:25}>: CallOnce<(&i32,)>` is not satisfied
   --> komadori-rayon/src/collector/unindexed_par_collector_base.rs:218:30
    |
218 |                   .filter_with(crate::ops::WithCloneStateParClosure::new(
    |  __________________-----------_^
    | |                  |
    | |                  required by a bound introduced by this call
219 | |                     sender,
220 | |                     clb!(for<'a, 'b> |sender: &'a mut Sender<i32>, num: &'b i32| -> bool {
221 | |                         if num % 2 == 0 {
...   |
227 | |                     }),
228 | |                 )),
    | |_________________^ unsatisfied trait bound
    |
help: the trait `for<'a, 'a> ops::call::CallOnce<(&'a i32,)>` is not implemented for `Callable<'a, Sender<i32>, {closure@lib.rs:433:25}>`
   --> komadori-rayon/src/ops/with_state_par_closure.rs:173:5
    |
173 |     pub struct Callable<'a, S, F> {
    |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
help: the trait `ops::call::CallOnce<Args>` is implemented for `ops::with_state_par_closure::call_once::Callable<'_, S, F>`
   --> komadori-rayon/src/ops/with_state_par_closure.rs:184:5
    |
184 | /     impl<S, F, Args> CallOnce<Args> for Callable<'_, S, F>
185 | |     where
186 | |         Args: PushFrontTuple,
187 | |         F: Call<Args::PushFront<S>>,
    | |____________________________________^
note: required for `WithCloneStateParClosure<Sender<i32>, {closure@lib.rs:433:25}>` to implement `for<'a> ops::par_fn::ParallelFnOnce<(&'a i32,)>`
   --> komadori-rayon/src/ops/par_fn.rs:31:18
    |
 31 | impl<F, Args, R> ParallelFnOnce<Args> for F
    |                  ^^^^^^^^^^^^^^^^^^^^     ^
...
 34 |     F: ParallelFnOnceBase<CallOnce: CallOnce<Args, Output = R>> + ?Sized,
    |                                     -------------------------- unsatisfied trait bound introduced here
note: required for `WithCloneStateParClosure<Sender<i32>, {closure@lib.rs:433:25}>` to implement `for<'a> ops::par_fn::ParallelFnMut<(&'a i32,)>`
   --> komadori-rayon/src/ops/par_fn.rs:55:15
    |
 55 | impl<F, Args> ParallelFnMut<Args> for F
    |               ^^^^^^^^^^^^^^^^^^^     ^
...
 58 |     F: ParallelFnOnce<Args> + ParallelFnMutBase<CallMut: CallMut<Args, Output = Self::Output>> + ?Sized,
    |                                                                        --------------------- unsatisfied trait bound introduced here
note: required by a bound in `collector::unindexed_par_collector_base::UnindexedParallelCollectorBase::filter_with`
   --> komadori-rayon/src/collector/unindexed_par_collector_base.rs:152:12
    |
149 |     fn filter_with<P, T>(self, pred: P) -> Filter<Self, P>
    |        ----------- required by a bound in this associated function
...
152 |         P: for<'a> ParallelFnMut<(&'a T,), Output = bool>,
    |            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ required by this bound in `UnindexedParallelCollectorBase::filter_with`
    = note: the full name for the type has been written to '/Users/ayakamain/Desktop/My/Programming/Rust/VS Code/library/komadori-rs/target/debug/deps/komadori_rayon-e0b19780b24b27b5.long-type-7508045992679520774.txt'
    = note: consider using `--verbose` to print the full type name to the console
```

But somehow, if we make a dedicated method instead, no compile error!

I don't know why, but `for<'a, 'a>` looks sus at least.
`clb!` is **not** the fault (confirmed).

And in some instances, I also got "this is the current limitation of the trait solver,"
so it could be a clue.

The best we can do now is `*()`, and the new `*_with()` being a unification of
old `*_with()` and `*_init()`.
Callers can "choose" either personality of the new `*_with()` by using
`()` for `local1` or `|| {}` for `local2_f`, or both if there is ever a case.
