use std::marker::PhantomData;

/// A `Layer` wraps `M`-typed `Stack<U>` to produce a `Stack<T>`
///
/// Some `Stack` types are dependendent on and generic over an inner `Stack`. For
/// example, a load balancer may implement `Stack<Authority>` and be
/// configured with a `Stack<SocketAddr>` that is used to build a service for
/// each enpdoint. Such a load balancer would provide a signature like:
///
/// ```ignore
/// impl<M: Stack<SocketAddr>> Layer<Authority, SocketAddr, M> for BalanceLayer<M> { ... }
/// ```
pub trait Layer<T, U, M: super::Stack<U>> {
    type Value;
    type Error;
    type Stack: super::Stack<T, Value = Self::Value, Error = Self::Error>;

    /// Produce a `Stack` value from a `M` value.
    fn bind(&self, next: M) -> Self::Stack;

    /// Compose this `Layer` with another.
    fn and_then<V, N, L>(self, inner: L)
        -> AndThen<U, Self, L>
    where
        N: super::Stack<V>,
        L: Layer<U, V, N>,
        Self: Layer<T, U, L::Stack> + Sized,
    {
        AndThen {
            outer: self,
            inner,
            _p: PhantomData,
        }
    }
}

/// Combines two `Layers` as one.
///
/// Given an `Outer: Layer<T, U, _>` and an `Inner: Layer<U, V, _>`, producesa
/// `Layer<T, C, _>`, encapsulating the logic of the Outer and Inner layers.
#[derive(Debug, Clone)]
pub struct AndThen<U, Outer, Inner>
{
    outer: Outer,
    inner: Inner,
    // `AndThen` should be Send/Sync independently of U`.
    _p: PhantomData<fn() -> U>,
}

impl<T, U, V, M, Outer, Inner> Layer<T, V, M>
    for AndThen<U, Outer, Inner>
where
    Outer: Layer<T, U, Inner::Stack>,
    Inner: Layer<U, V, M>,
    M: super::Stack<V>,
{
    type Value = Outer::Value;
    type Error = Outer::Error;
    type Stack = Outer::Stack;

    fn bind(&self, next: M) -> Self::Stack {
        self.outer.bind(self.inner.bind(next))
    }
}
