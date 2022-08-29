use crate::AsyncBody;

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use http::HeaderMap;
use http_body::Body;

use self_reference::{RefDef, SelfReference};


#[pin_project::pin_project]
pub struct AsyncBodyAdapter<B>
where
    B: AsyncBody + 'static,
{
    #[pin]
    state: SelfReference<B, AdapterStateRefDef<B>>,
}

impl<B> AsyncBodyAdapter<B>
where
    B: AsyncBody,
{
    pub fn new(body: B) -> Self {
        Self {
            state: SelfReference::new(body, || AdapterState::Empty),
        }
    }
}

enum AdapterState<'a, B>
where
    B: AsyncBody + 'a,
{
    ProcessingData(Pin<Box<dyn Future<Output = Option<Result<B::Data, B::Error>>> + 'a>>),
    ProcessingTrailers(Pin<Box<dyn Future<Output = Result<Option<HeaderMap>, B::Error>> + 'a>>),
    Empty,
}

impl<'a, B> AdapterState<'a, B>
where
    B: AsyncBody + 'a,
{
    pub fn is_empty(&self) -> bool {
        if let AdapterState::Empty = self {
            true
        } else {
            false
        }
    }
}

// Dummy object to provide lifetime gateway for AdapaterState.
struct AdapterStateRefDef<B>(core::marker::PhantomData<B>);
impl<'this, B> RefDef<'this> for AdapterStateRefDef<B>
where
    for<'a> B: AsyncBody + 'a,
{
    type Type = AdapterState<'this, B>;
}

impl<B> Body for AsyncBodyAdapter<B>
where
    for<'a> B: AsyncBody + 'a,
{
    type Data = B::Data;
    type Error = B::Error;

    fn poll_data(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Self::Data, Self::Error>>> {
        let mut proj = self.project();

        let state = proj.state.as_mut().pin_mut();
        if state.is_empty() {
            proj.state
                .as_mut()
                .reset(|v| AdapterState::ProcessingData(v.data()));
        }

        let mut state = proj.state.pin_mut();
        if let AdapterState::ProcessingData(fut) = state.as_mut().get_mut() {
            return match fut.as_mut().poll(cx) {
                Poll::Ready(v) => {
                    // Future has been polled, which means we consumed body reference.
                    // to re-initialize body reference. set as empty.
                    state.set(AdapterState::Empty);
                    Poll::Ready(v)
                }
                Poll::Pending => Poll::Pending,
            };
        }

        unreachable!("bad async body adapter state while polling data!");
    }

    fn poll_trailers(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<Option<HeaderMap>, Self::Error>> {
        let mut proj = self.project();

        let state = proj.state.as_mut().pin_mut();
        if state.is_empty() {
            proj.state
                .as_mut()
                .reset(|v| AdapterState::ProcessingTrailers(v.trailers()));
        }

        let mut state = proj.state.pin_mut();
        if let AdapterState::ProcessingTrailers(fut) = state.as_mut().get_mut() {
            return match fut.as_mut().poll(cx) {
                Poll::Ready(v) => {
                    // Future has been polled, which means we consumed body reference.
                    // to re-initialize body reference. set as empty.
                    state.set(AdapterState::Empty);
                    Poll::Ready(v)
                }
                Poll::Pending => Poll::Pending,
            };
        }

        unreachable!("bad async body adapter state while polling data!");
    }
}
