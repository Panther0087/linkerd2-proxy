//! Layer to map service errors into responses.

#![deny(warnings, rust_2018_idioms)]

use futures::{Async, Future, Poll};
use linkerd2_error::Error;

/// Creates an error responder for a request.
pub trait NewRespond<Req, Rsp, E = Error> {
    type Response;
    type Respond: Respond<Rsp, E, Response = Self::Response>;

    fn new_respond(&self, req: &Req) -> Self::Respond;
}

/// Creates a response for an error.
pub trait Respond<Rsp, E = Error> {
    type Response;
    fn respond(&self, response: Result<Rsp, E>) -> Result<Self::Response, E>;
}

#[derive(Clone, Debug)]
pub struct RespondLayer<N> {
    new_respond: N,
}

#[derive(Clone, Debug)]
pub struct RespondService<N, S> {
    new_respond: N,
    inner: S,
}

#[derive(Debug)]
pub struct RespondFuture<R, F> {
    respond: R,
    inner: F,
}

impl<N: Clone> RespondLayer<N> {
    pub fn new(new_respond: N) -> Self {
        Self { new_respond }
    }
}

impl<N: Clone, S> tower::layer::Layer<S> for RespondLayer<N> {
    type Service = RespondService<N, S>;

    fn layer(&self, inner: S) -> Self::Service {
        RespondService {
            inner,
            new_respond: self.new_respond.clone(),
        }
    }
}

impl<Req, N, S> tower::Service<Req> for RespondService<N, S>
where
    S: tower::Service<Req>,
    N: NewRespond<Req, S::Response, S::Error>,
{
    type Response = N::Response;
    type Error = S::Error;
    type Future = RespondFuture<N::Respond, S::Future>;

    fn poll_ready(&mut self) -> Poll<(), Self::Error> {
        self.inner.poll_ready()
    }

    fn call(&mut self, req: Req) -> Self::Future {
        let respond = self.new_respond.new_respond(&req);
        let inner = self.inner.call(req);
        RespondFuture { respond, inner }
    }
}

impl<R, F> Future for RespondFuture<R, F>
where
    F: Future,
    R: Respond<F::Item, F::Error>,
{
    type Item = R::Response;
    type Error = F::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.inner.poll() {
            Ok(Async::NotReady) => Ok(Async::NotReady),
            Ok(Async::Ready(rsp)) => self.respond.respond(Ok(rsp)).map(Async::Ready),
            Err(err) => self.respond.respond(Err(err)).map(Async::Ready),
        }
    }
}
