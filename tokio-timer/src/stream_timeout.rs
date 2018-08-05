use Delay;

use futures::{Stream, Future, Poll, Async};

use std::error;
use std::fmt;
use std::time::{Instant, Duration};

/// Allows a given `Stream` to execute until it has not been ready for the duration
/// of the timeout.
#[must_use = "streams do nothing unless polled"]
#[derive(Debug)]
pub struct StreamTimeout<T> {
    stream: T,
    timeout: Duration,
    delay: Delay,
}

/// Error returned by `StreamTimeout` stream.
#[derive(Debug)]
pub struct StreamTimeoutError<T>(Kind<T>);

/// StreamTimeout error variants
#[derive(Debug)]
enum Kind<T> {
    /// Inner future returned an error
    Inner(T),

    /// Timer returned an error.
    Timer(::Error),
}

impl<T> StreamTimeout<T> {
    /// Create a new `StreamTimeout` that completes when `stream` completes or when
    /// `stream` hasn't been ready for the `timout` duration
    pub fn new(stream: T, timeout: Duration) -> StreamTimeout<T> {
        StreamTimeout {
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

impl<T> Stream for StreamTimeout<T>
where T: Stream,
{
    type Item = T::Item;
    type Error = StreamTimeoutError<T::Error>;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        // First, try polling the future
        match self.stream.poll() {
            Ok(Async::Ready(v)) => {
                // reset the delay when we receive a ready
                self.reset_delay();
                return Ok(Async::Ready(v))
            },
            Ok(Async::NotReady) => {}
            Err(e) => return Err(StreamTimeoutError::inner(e)),
        }

        // Now check the timer
        match self.delay.poll() {
            Ok(Async::NotReady) => Ok(Async::NotReady),
            Ok(Async::Ready(_)) => {
                // Send none to signal that there hasn't been a ready poll
                // after the timeout so we can probably stop.
                Ok(Async::Ready(None))
            },
            Err(e) => Err(StreamTimeoutError::timer(e)),
        }
    }
}

// ===== impl StreamTimeoutError =====

impl<T> StreamTimeoutError<T> {
    /// Create a new `StreamTimeoutError` representing the inner future completing
    /// with `Err`.
    pub fn inner(err: T) -> StreamTimeoutError<T> {
        StreamTimeoutError(Kind::Inner(err))
    }

    /// Returns `true` if the error was caused by the inner stream completing
    /// with `Err`.
    pub fn is_inner(&self) -> bool {
        match self.0 {
            Kind::Inner(_) => true,
            _ => false,
        }
    }

    /// Consumes `self`, returning the inner future error.
    pub fn into_inner(self) -> Option<T> {
        match self.0 {
            Kind::Inner(err) => Some(err),
            _ => None,
        }
    }

    /// Creates a new `StreamTimeoutError` representing an error encountered by the
    /// timer implementation
    pub fn timer(err: ::Error) -> StreamTimeoutError<T> {
        StreamTimeoutError(Kind::Timer(err))
    }

    /// Returns `true` if the error was caused by the timer.
    pub fn is_timer(&self) -> bool {
        match self.0 {
            Kind::Timer(_) => true,
            _ => false,
        }
    }

    /// Consumes `self`, returning the error raised by the timer implementation.
    pub fn into_timer(self) -> Option<::Error> {
        match self.0 {
            Kind::Timer(err) => Some(err),
            _ => None,
        }
    }
}

impl<T: error::Error> error::Error for StreamTimeoutError<T> {
    fn description(&self) -> &str {
        use self::Kind::*;

        match self.0 {
            Inner(ref e) => e.description(),
            Timer(ref e) => e.description(),
        }
    }
}

impl<T: fmt::Display> fmt::Display for StreamTimeoutError<T> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        use self::Kind::*;

        match self.0 {
            Inner(ref e) => e.fmt(fmt),
            Timer(ref e) => e.fmt(fmt),
        }
    }
}
