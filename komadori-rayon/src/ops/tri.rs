mod trying;

pub use trying::*;

use core::{convert::Infallible, ops::ControlFlow};

pub trait Try {
    type Output;
    type Residual: Residual;

    fn from_output(output: Self::Output) -> Self;
    fn from_residual(residual: Self::Residual) -> Self;
    fn branch(self) -> ControlFlow<Self::Residual, Self::Output>;
}

pub trait Residual: Sized {
    type TryType<O>: Try<Output = O, Residual = Self>;
}

impl<B, C> Try for ControlFlow<B, C> {
    type Output = C;
    type Residual = ControlFlow<B, Infallible>;

    #[inline]
    fn from_output(output: Self::Output) -> Self {
        ControlFlow::Continue(output)
    }

    #[inline]
    fn from_residual(residual: Self::Residual) -> Self {
        match residual {
            ControlFlow::Break(b) => ControlFlow::Break(b),
        }
    }

    #[inline]
    fn branch(self) -> ControlFlow<Self::Residual, Self::Output> {
        match self {
            ControlFlow::Continue(c) => ControlFlow::Continue(c),
            ControlFlow::Break(b) => ControlFlow::Break(ControlFlow::Break(b)),
        }
    }
}

impl<T> Try for Option<T> {
    type Output = T;
    type Residual = Option<Infallible>;

    #[inline]
    fn from_output(output: Self::Output) -> Self {
        Some(output)
    }

    #[inline]
    fn from_residual(residual: Self::Residual) -> Self {
        match residual {
            None => None,
        }
    }

    #[inline]
    fn branch(self) -> ControlFlow<Self::Residual, Self::Output> {
        match self {
            Some(c) => ControlFlow::Continue(c),
            None => ControlFlow::Break(None),
        }
    }
}

impl<T, E> Try for Result<T, E> {
    type Output = T;
    type Residual = Result<Infallible, E>;

    #[inline]
    fn from_output(output: Self::Output) -> Self {
        Ok(output)
    }

    #[inline]
    #[track_caller]
    fn from_residual(residual: Self::Residual) -> Self {
        match residual {
            Err(e) => Err(e),
        }
    }

    #[inline]
    fn branch(self) -> ControlFlow<Self::Residual, Self::Output> {
        match self {
            Ok(c) => ControlFlow::Continue(c),
            Err(e) => ControlFlow::Break(Err(e)),
        }
    }
}

impl<B> Residual for ControlFlow<B, Infallible> {
    type TryType<C> = ControlFlow<B, C>;
}

impl Residual for Option<Infallible> {
    type TryType<T> = Option<T>;
}

impl<E> Residual for Result<Infallible, E> {
    type TryType<T> = Result<T, E>;
}

pub type ChangeOutputType<T, O> = <<T as Try>::Residual as Residual>::TryType<O>;
