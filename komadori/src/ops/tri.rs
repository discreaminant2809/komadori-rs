use std::{convert::Infallible, ops::ControlFlow, task::Poll};

pub trait Try {
    type Output;
    type Residual;

    fn from_output(output: Self::Output) -> Self;
    fn from_residual(residual: Self::Residual) -> Self;
    fn branch(self) -> ControlFlow<Self::Residual, Self::Output>;
}

// pub trait Residual<O>: Sized {
//     type TryType: Try<Output = O, Residual = Self>;
// }

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

impl<T, E> Try for Poll<Result<T, E>> {
    type Output = Poll<T>;
    type Residual = Result<Infallible, E>;

    #[inline]
    fn from_output(output: Self::Output) -> Self {
        output.map(Ok)
    }

    #[inline]
    fn from_residual(residual: Self::Residual) -> Self {
        match residual {
            Err(e) => Poll::Ready(Err(e)),
        }
    }

    #[inline]
    fn branch(self) -> ControlFlow<Self::Residual, Self::Output> {
        match self {
            Poll::Pending => ControlFlow::Continue(Poll::Pending),
            Poll::Ready(Ok(c)) => ControlFlow::Continue(Poll::Ready(c)),
            Poll::Ready(Err(e)) => ControlFlow::Break(Err(e)),
        }
    }
}

impl<T, E> Try for Poll<Option<Result<T, E>>> {
    type Output = Poll<Option<T>>;
    type Residual = Result<Infallible, E>;

    #[inline]
    fn from_output(output: Self::Output) -> Self {
        match output {
            Poll::Ready(o) => Poll::Ready(o.map(Ok)),
            Poll::Pending => Poll::Pending,
        }
    }

    #[inline]
    fn from_residual(residual: Self::Residual) -> Self {
        match residual {
            Err(e) => Poll::Ready(Some(Err(e))),
        }
    }

    #[inline]
    fn branch(self) -> ControlFlow<Self::Residual, Self::Output> {
        match self {
            Poll::Pending => ControlFlow::Continue(Poll::Pending),
            Poll::Ready(None) => ControlFlow::Continue(Poll::Ready(None)),
            Poll::Ready(Some(Ok(c))) => ControlFlow::Continue(Poll::Ready(Some(c))),
            Poll::Ready(Some(Err(e))) => ControlFlow::Break(Err(e)),
        }
    }
}

// impl<B, C> Residual<C> for ControlFlow<B, Infallible> {
//     type TryType = ControlFlow<B, C>;
// }

// impl<T> Residual<T> for Option<Infallible> {
//     type TryType = Option<T>;
// }

// impl<T, E> Residual<T> for Result<Infallible, E> {
//     type TryType = Result<T, E>;
// }
