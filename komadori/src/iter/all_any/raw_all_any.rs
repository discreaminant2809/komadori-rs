use std::ops::ControlFlow;

#[derive(Clone)]
// ALL = true: ALL mode
// ALL = false: ANY mode
pub(super) struct RawAllAny<F, const ALL: bool> {
    pred: Option<F>,
}

impl<F, const ALL: bool> RawAllAny<F, ALL> {
    #[inline]
    pub const fn new(pred: F) -> Self {
        Self { pred: Some(pred) }
    }

    #[inline]
    pub const fn get(&self) -> bool {
        // is_none/ALL 0 (ANY) 1 (ALL)
        // 0           0       1
        // 1           1       0
        // => XOR
        ALL ^ self.pred.is_none()
    }

    pub fn collect_impl(&mut self, f: impl FnOnce(&mut F) -> bool) -> ControlFlow<()> {
        let Some(ref mut pred) = self.pred else {
            return ControlFlow::Break(());
        };

        // f/ALL 0 (ANY) 1 (ALL)
        // 0     0       1
        // 1     1       0
        // => XOR
        if ALL ^ f(pred) {
            // Found
            self.pred = None;
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    }

    pub fn collect_then_finish_impl(self, f: impl FnOnce(F) -> bool) -> bool {
        // f/ALL 0 (ANY) 1 (ALL)
        // 0     0       0
        // 1     1       1
        // => f (only depends on f)
        self.pred.map_or(!ALL, f)
    }

    #[inline]
    pub fn debug_impl(&self, mut f: std::fmt::DebugStruct<'_, '_>) -> std::fmt::Result {
        f
            // We exclude all fields containing closures/markers,
            // but then we're left with nothing.
            // So, we trick outside that we have a
            // "phantom" state tracking the accumulation result.
            .field(if ALL { "all" } else { "any" }, &self.get())
            .field("f", &std::any::type_name::<F>())
            .finish()
    }

    #[inline]
    pub fn break_hint(&self) -> ControlFlow<()> {
        if self.pred.is_some() {
            ControlFlow::Continue(())
        } else {
            ControlFlow::Break(())
        }
    }
}
