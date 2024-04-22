use std::any::Any;
use std::async_iter::AsyncIterator;
use std::fmt;
use std::future::Future;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::pin::Pin;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::*;
use std::sync::{Condvar, Mutex, MutexGuard};
use std::task::ready;
use std::task::{Context, Poll};
use std::thread;
use std::time::Duration;

use async_channel::{bounded, Receiver};
use futures_core::Stream;
use futures_io::{AsyncRead, AsyncSeek, AsyncWrite};
use piper::{pipe, Reader, Writer};

use crate::future::poll_fn;
use crate::spawn;
use crate::TaskHandle;

enum State<T> {
    Idle(Option<Box<T>>),
    WithMut(TaskHandle<Box<T>>),
    Streaming(Option<Box<dyn Any + Send + Sync>>, TaskHandle<Box<T>>),
    Reading(Option<Reader>, TaskHandle<(io::Result<()>, Box<T>)>),
    Writing(Option<Writer>, TaskHandle<(io::Result<()>, Box<T>)>),
    Seeking(TaskHandle<(SeekFrom, io::Result<u64>, Box<T>)>),
}

pub struct Unblock<T> {
    state: State<T>,
    cap: Option<usize>,
}

impl<T> Unblock<T> {
    pub fn new(io: T) -> Self {
        Self {
            state: State::Idle(Some(Box::new(io))),
            cap: None,
        }
    }

    pub fn with_capacity(cap: usize, io: T) -> Self {
        Self {
            state: State::Idle(Some(Box::new(io))),
            cap: Some(cap),
        }
    }

    pub async fn get_mut(&mut self) -> &mut T {
        poll_fn(|cx| self.poll_stop(cx)).await.ok();

        if let State::Idle(t) = &mut self.state {
            t.as_mut().expect("inner value was taken out")
        } else {
            unreachable!("when stopped, the state machine must be in idle state")
        }
    }

    pub async fn with_mut<R, F>(&mut self, op: F) -> R
    where
        F: FnOnce(&mut T) -> R + Send + 'static,
        R: Send + 'static,
        T: Send + 'static,
    {
        poll_fn(|cx| self.poll_stop(cx)).await.ok();

        let mut t = if let State::Idle(t) = &mut self.state {
            t.take().expect("inner value was taken out")
        } else {
            unreachable!("when stopped, the state machine must be in idle state")
        };

        let (sender, receiver) = bounded(1);
        let handle = spawn(async move {
            sender.try_send(op(&mut t)).ok();
            t
        });
        self.state = State::WithMut(handle);

        receiver
            .recv()
            .await
            .expect("`Unblock::with_mut()` operation has panicked")
    }

    pub async fn into_inner(mut self) -> T {
        // let mut this = self;

        poll_fn(|cx| self.poll_stop(cx)).await.ok();

        if let State::Idle(t) = &mut self.state {
            *t.take().expect("inner value was taken out")
        } else {
            unreachable!("when stopped, the state machine must be in idle state")
        }
    }

    fn poll_stop(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        loop {
            match &mut self.state {
                State::Idle(_) => return Poll::Ready(Ok(())),
                State::WithMut(task) => {
                    let io = ready!(Pin::new(task).poll(cx));
                    self.state = State::Idle(Some(io));
                }
                State::Streaming(any, task) => {
                    any.take();

                    let iter = ready!(Pin::new(task).poll(cx));
                    self.state = State::Idle(Some(iter));
                }
                State::Reading(reader, task) => {
                    reader.take();

                    let (res, io) = ready!(Pin::new(task).poll(cx));
                    self.state = State::Idle(Some(io));
                    res?;
                }
                State::Writing(writer, task) => {
                    writer.take();

                    let (res, io) = ready!(Pin::new(task).poll(cx));
                    self.state = State::Idle(Some(io));
                    res?;
                }
                State::Seeking(task) => {
                    let (_, res, io) = ready!(Pin::new(task).poll(cx));
                    self.state = State::Idle(Some(io));
                    res?;
                }
            }
        }
    }
}

impl<T: Iterator + Send + 'static> Stream for Unblock<T>
where
    T::Item: Send + 'static,
{
    type Item = T::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            match &mut self.state {
                State::WithMut(..)
                | State::Streaming(None, _)
                | State::Reading(..)
                | State::Writing(..)
                | State::Seeking(..) => {
                    ready!(self.poll_stop(cx)).ok();
                }
                State::Idle(iter) => {
                    let mut iter = iter.take().expect("inner iterator was taken out");

                    let (sender, receiver) = bounded(self.cap.unwrap_or(8 * 1024)); // 8192 items

                    // Spawn a blocking task that runs the iterator and returns it when done.
                    let handle = spawn(async move {
                        for item in &mut iter {
                            if sender.send(item).await.is_err() {
                                break;
                            }
                        }
                        iter
                    });

                    self.state = State::Streaming(Some(Box::new(Box::pin(receiver))), handle);
                }
                State::Streaming(Some(any), task) => {
                    let receiver = any.downcast_mut::<Pin<Box<Receiver<T::Item>>>>().unwrap();

                    let opt = ready!(receiver.as_mut().poll_next(cx));

                    if opt.is_none() {
                        let iter = ready!(Pin::new(task).poll(cx));
                        self.state = State::Idle(Some(iter));
                    }

                    return Poll::Ready(opt);
                }
            }
        }
    }
}

impl<T: Read + Send + 'static> AsyncRead for Unblock<T> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        loop {
            match &mut self.state {
                State::WithMut(..)
                | State::Reading(None, _)
                | State::Streaming(..)
                | State::Writing(..)
                | State::Seeking(..) => {
                    ready!(self.poll_stop(cx))?;
                }

                State::Idle(io) => {
                    let mut io = io.take().expect("inner value was taken out");

                    let (reader, mut writer) = pipe(self.cap.unwrap_or(8 * 1024 * 1024)); // 8 MB

                    let task = spawn(async move {
                        loop {
                            match poll_fn(|cx| writer.poll_fill(cx, &mut io)).await {
                                Ok(0) => return (Ok(()), io),
                                Ok(_) => {}
                                Err(err) => return (Err(err), io),
                            }
                        }
                    });

                    self.state = State::Reading(Some(reader), task);
                }

                State::Reading(Some(reader), task) => {
                    let n = ready!(reader.poll_drain(cx, buf))?;

                    if n == 0 {
                        let (res, io) = ready!(Pin::new(task).poll(cx));
                        self.state = State::Idle(Some(io));
                        res?;
                    }

                    return Poll::Ready(Ok(n));
                }
            }
        }
    }
}

impl<T: Write + Send + 'static> AsyncWrite for Unblock<T> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        loop {
            match &mut self.state {
                State::WithMut(..)
                | State::Writing(None, _)
                | State::Streaming(..)
                | State::Reading(..)
                | State::Seeking(..) => {
                    ready!(self.poll_stop(cx))?;
                }

                State::Idle(io) => {
                    let mut io = io.take().expect("inner value was taken out");

                    let (mut reader, writer) = pipe(self.cap.unwrap_or(8 * 1024 * 1024)); // 8 MB

                    let task = spawn(async move {
                        loop {
                            match poll_fn(|cx| reader.poll_drain(cx, &mut io)).await {
                                Ok(0) => return (io.flush(), io),
                                Ok(_) => {}
                                Err(err) => {
                                    io.flush().ok();
                                    return (Err(err), io);
                                }
                            }
                        }
                    });

                    self.state = State::Writing(Some(writer), task);
                }

                State::Writing(Some(writer), _) => return writer.poll_fill(cx, buf),
            }
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        loop {
            match &mut self.state {
                State::WithMut(..)
                | State::Streaming(..)
                | State::Writing(..)
                | State::Reading(..)
                | State::Seeking(..) => {
                    ready!(self.poll_stop(cx))?;
                }
                State::Idle(_) => return Poll::Ready(Ok(())),
            }
        }
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        ready!(Pin::new(&mut self).poll_flush(cx))?;

        self.state = State::Idle(None);
        Poll::Ready(Ok(()))
    }
}

impl<T: Seek + Send + 'static> AsyncSeek for Unblock<T> {
    fn poll_seek(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        pos: SeekFrom,
    ) -> Poll<io::Result<u64>> {
        loop {
            match &mut self.state {
                State::WithMut(..)
                | State::Streaming(..)
                | State::Reading(..)
                | State::Writing(..) => {
                    ready!(self.poll_stop(cx))?;
                }

                State::Idle(io) => {
                    let mut io = io.take().expect("inner value was taken out");

                    let task = spawn(async move {
                        let res = io.seek(pos);
                        (pos, res, io)
                    });
                    self.state = State::Seeking(task);
                }

                State::Seeking(task) => {
                    let (original_pos, res, io) = ready!(Pin::new(task).poll(cx));
                    self.state = State::Idle(Some(io));
                    let current = res?;

                    if original_pos == pos {
                        return Poll::Ready(Ok(current));
                    }
                }
            }
        }
    }
}

impl<T: fmt::Debug> fmt::Debug for Unblock<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut f = f.debug_struct("Unblock");
        match &self.state {
            State::Idle(None) => f.field("io", &"<closed>"),
            State::Idle(Some(io)) => f.field("io", io.as_ref()),
            _ => f.field("io", &"<blocked>"),
        }
        .finish()
    }
}
