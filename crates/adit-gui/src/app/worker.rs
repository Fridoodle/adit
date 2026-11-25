//! A background thread that runs the latest request only (U-APP-3: renders off the UI thread).

use eframe::egui;
use std::sync::mpsc::{Receiver, Sender, channel};

pub struct Worker<Q, R> {
    tx: Sender<(u64, Q)>,
    rx: Receiver<(u64, R)>,
    sent: u64,
    shown: u64,
}

impl<Q: Send + 'static, R: Send + 'static> Worker<Q, R> {
    pub fn new(ctx: &egui::Context, f: impl Fn(Q) -> R + Send + 'static) -> Self {
        let (tx, jobs) = channel::<(u64, Q)>();
        let (done, rx) = channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            while let Ok(mut job) = jobs.recv() {
                // Requests that piled up while rendering are stale: keep the newest.
                while let Ok(j) = jobs.try_recv() {
                    job = j;
                }
                let r = f(job.1);
                if done.send((job.0, r)).is_err() {
                    return;
                }
                ctx.request_repaint();
            }
        });
        Worker {
            tx,
            rx,
            sent: 0,
            shown: 0,
        }
    }

    pub fn request(&mut self, q: Q) {
        self.sent += 1;
        let _ = self.tx.send((self.sent, q));
    }

    /// A request has no result yet.
    pub fn busy(&self) -> bool {
        self.shown < self.sent
    }

    /// The newest result not shown yet; older ones are dropped.
    pub fn latest(&mut self) -> Option<R> {
        let mut out = None;
        while let Ok((n, r)) = self.rx.try_recv() {
            if n > self.shown {
                self.shown = n;
                out = Some(r);
            }
        }
        out
    }
}
