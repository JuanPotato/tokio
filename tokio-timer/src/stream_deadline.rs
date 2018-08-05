use {Delay, DeadlineError};

use futures::{Stream, Future, Poll, Async};

use std::time::{Instant, Duration};


/// Allows a given `Stream` to execute until it has not been ready for the duration
/// of the timeout.
#[must_use = "streams do nothing unless polled"]
#[derive(Debug)]
pub struct StreamDeadline<T> {
    stream: T,
    timeout: Duration,
    delay: Delay,
}

impl<T> StreamDeadline<T> {
    /// Create a new `StreamDeadline` that completes when `stream` completes or when
    /// `stream` hasn't been ready for the `timout` duration
    pub fn new(stream: T, timeout: Duration) -> StreamDeadline<T> {
        StreamDeadline {
            stream,
            timeout,
            delay: Delay::new(Instant::now() + timeout),
        }
    }

    /// Gets a reference to the underlying stream in this deadline.
    pub fn get_ref(&self) -> &T {
        &self.stream
    }

    /// Gets a mutable reference to the underlying stream in this deadline.
    pub fn get_mut(&mut self) -> &mut T {
        &mut self.stream
    }

    /// Consumes this deadline, returning the underlying stream.
    pub fn into_inner(self) -> T {
        self.stream
    }

    fn reset_delay(&mut self) {
        self.delay = Delay::new(Instant::now() + self.timeout);
    }
}

impl<T> Stream for StreamDeadline<T>
where T: Stream,
{
    type Item = T::Item;
    type Error = DeadlineError<T::Error>;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        // First, try polling the future
        match self.stream.poll() {
            Ok(Async::Ready(v)) => {
                // reset the delay when we receive a ready
                self.reset_delay();
                return Ok(Async::Ready(v))
            },
            Ok(Async::NotReady) => {}
            Err(e) => return Err(DeadlineError::inner(e)),
        }

        // Now check the timer
        match self.delay.poll() {
            Ok(Async::NotReady) => Ok(Async::NotReady),
            Ok(Async::Ready(_)) => {
                // Send none to signal that there hasn't been a ready poll
                // after the timeout so we can probably stop.
                Ok(Async::Ready(None))
            },
            Err(e) => Err(DeadlineError::timer(e)),
        }
    }
}
