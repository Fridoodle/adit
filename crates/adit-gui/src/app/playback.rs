//! Playback clock, audio output and meters (U-APP-7).

use crate::UiState;
use adit_core::{FLICKS_PER_SECOND, Id, Project, Time};
use adit_mcp::Session;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Default)]
pub struct Playback {
    /// Wall clock and playhead when playback (at this shuttle speed) started.
    anchor: Option<(Instant, Time, i32)>,
    /// Playhead this clock last set; anything else means the user moved it.
    last: Time,
    audio: Option<Audio>,
    /// What the mixer is playing: (project, revision, sequence).
    audio_key: Option<(String, u64, Id)>,
    /// Displayed meter peaks (linear), decaying.
    pub peaks: [f32; 2],
}

impl Playback {
    pub fn tick(
        &mut self,
        session: &Session,
        pid: &str,
        rev: u64,
        project: &Arc<Project>,
        ui: &mut UiState,
        status: &mut String,
    ) {
        let decay = |p: f32| p * 0.85;
        self.peaks = self.peaks.map(decay);
        if let Some(a) = &self.audio {
            for (shown, live) in self.peaks.iter_mut().zip(a.peaks.iter()) {
                *shown = shown.max(f32::from_bits(live.swap(0, Ordering::Relaxed)));
            }
            while let Ok(e) = a.errors.try_recv() {
                *status = e;
            }
        }
        let Some(seq) = project.sequence(ui.sequence_id).filter(|_| ui.playing) else {
            self.stop();
            return;
        };
        let now = Instant::now();
        let anchor = match self.anchor {
            Some(a) if a.2 == ui.shuttle && ui.playhead == self.last => a,
            _ => (now, ui.playhead, ui.shuttle),
        };
        self.anchor = Some(anchor);
        let elapsed = now.duration_since(anchor.0).as_secs_f64();
        let mut t = anchor.1 + (elapsed * anchor.2 as f64 * FLICKS_PER_SECOND as f64) as Time;
        let end = super::frame(seq.duration(), seq.fps);
        if (ui.shuttle > 0 && t >= end) || (ui.shuttle < 0 && t <= 0) {
            t = t.clamp(0, end);
            ui.playing = false;
            ui.shuttle = 0;
        }
        ui.playhead = super::frame(t.max(0), seq.fps);
        self.last = ui.playhead;

        // Sound only at normal speed forward.
        let want = (ui.playing && ui.shuttle == 1).then(|| (pid.to_string(), rev, seq.id));
        if want != self.audio_key {
            match &want {
                Some(_) => {
                    let a = self
                        .audio
                        .get_or_insert_with(|| Audio::new(session.clone()));
                    let _ = a.tx.send(Msg::Play(project.clone(), seq.id, ui.playhead));
                }
                None => self.silence(),
            }
            self.audio_key = want;
        }
        if !ui.playing {
            self.stop();
        }
    }

    pub fn stop(&mut self) {
        self.anchor = None;
        if self.audio_key.take().is_some() {
            self.silence();
        }
    }

    fn silence(&self) {
        if let Some(a) = &self.audio {
            let _ = a.tx.send(Msg::Stop);
        }
    }
}

enum Msg {
    /// Restart mixing this snapshot from this time.
    // shortcut: an edit during playback restarts the mix at the playhead (small glitch); A/V drift is not corrected.
    Play(Arc<Project>, Id, Time),
    Stop,
}

/// The output stream and the mixer feeding it, on one thread (a cpal stream is not `Send`).
struct Audio {
    tx: Sender<Msg>,
    peaks: Arc<[AtomicU32; 2]>,
    errors: Receiver<String>,
}

type Buffer = Arc<Mutex<VecDeque<f32>>>;

impl Audio {
    fn new(session: Session) -> Self {
        let (tx, rx) = channel();
        let (etx, errors) = channel();
        let peaks: Arc<[AtomicU32; 2]> = Arc::new([AtomicU32::new(0), AtomicU32::new(0)]);
        let p2 = peaks.clone();
        std::thread::spawn(move || mixer(session, rx, etx, p2));
        Audio { tx, peaks, errors }
    }
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

fn mixer(session: Session, rx: Receiver<Msg>, errors: Sender<String>, peaks: Arc<[AtomicU32; 2]>) {
    let buf: Buffer = Arc::default();
    let out = match open_output(buf.clone(), peaks) {
        Ok(o) => Some(o),
        Err(e) => {
            let _ = errors.send(format!("no audio output ({e}); playback is silent"));
            None
        }
    };
    let mut playing: Option<(Arc<Project>, Id, Time)> = None;
    loop {
        match rx.recv_timeout(Duration::from_millis(40)) {
            Ok(Msg::Play(p, seq, from)) => {
                lock(&buf).clear();
                playing = Some((p, seq, from));
            }
            Ok(Msg::Stop) => {
                lock(&buf).clear();
                playing = None;
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
        let (Some((_, rate)), Some((p, seq, cursor))) = (&out, &mut playing) else {
            continue;
        };
        let Some(seq_rate) = p.sequence(*seq).map(|s| s.sample_rate) else {
            continue;
        };
        // Keep ~0.5 s mixed ahead, in 0.25 s chunks.
        while lock(&buf).len() < *rate as usize {
            let chunk = FLICKS_PER_SECOND / 4;
            match session.mix(p, *seq, *cursor, chunk) {
                Ok(s) => {
                    let s = resample(&s, seq_rate, *rate);
                    lock(&buf).extend(s);
                    *cursor += chunk;
                }
                Err(e) => {
                    let _ = errors.send(format!("audio: {e}"));
                    playing = None;
                    break;
                }
            }
        }
    }
}

/// Interleaved stereo, linear interpolation.
// shortcut: per-chunk linear resampling (tiny seams at chunk edges), use a real resampler if it is audible.
fn resample(s: &[f32], from: u32, to: u32) -> Vec<f32> {
    if from == to || from == 0 {
        return s.to_vec();
    }
    let n = s.len() / 2;
    let m = (n as u64 * to as u64 / from as u64) as usize;
    let mut out = Vec::with_capacity(m * 2);
    for i in 0..m {
        let x = i as f64 * from as f64 / to as f64;
        let (j, f) = (x as usize, (x - x.floor()) as f32);
        for c in 0..2 {
            let a = s.get(j * 2 + c).copied().unwrap_or(0.0);
            let b = s.get((j + 1) * 2 + c).copied().unwrap_or(a);
            out.push(a + (b - a) * f);
        }
    }
    out
}

/// Opens the default output device; returns the stream and its sample rate.
fn open_output(buf: Buffer, peaks: Arc<[AtomicU32; 2]>) -> Result<(cpal::Stream, u32), String> {
    let device = cpal::default_host()
        .default_output_device()
        .ok_or("no output device")?;
    let config = device.default_output_config().map_err(|e| e.to_string())?;
    let rate = config.sample_rate();
    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => build::<f32>(&device, config.into(), buf, peaks),
        cpal::SampleFormat::I16 => build::<i16>(&device, config.into(), buf, peaks),
        cpal::SampleFormat::U16 => build::<u16>(&device, config.into(), buf, peaks),
        cpal::SampleFormat::I32 => build::<i32>(&device, config.into(), buf, peaks),
        f => return Err(format!("unsupported sample format {f}")),
    }?;
    stream.play().map_err(|e| e.to_string())?;
    Ok((stream, rate))
}

fn build<T: cpal::SizedSample + cpal::FromSample<f32>>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    buf: Buffer,
    peaks: Arc<[AtomicU32; 2]>,
) -> Result<cpal::Stream, String> {
    let channels = config.channels as usize;
    device
        .build_output_stream(
            config,
            move |out: &mut [T], _: &cpal::OutputCallbackInfo| {
                let mut b = lock(&buf);
                let mut peak = [0f32; 2];
                for frame in out.chunks_mut(channels.max(1)) {
                    let l = b.pop_front().unwrap_or(0.0);
                    let r = b.pop_front().unwrap_or(0.0);
                    peak = [peak[0].max(l.abs()), peak[1].max(r.abs())];
                    for (i, x) in frame.iter_mut().enumerate() {
                        let v = match (channels, i) {
                            (1, _) => (l + r) / 2.0,
                            (_, 0) => l,
                            (_, 1) => r,
                            _ => 0.0,
                        };
                        *x = T::from_sample(v);
                    }
                }
                for (a, p) in peaks.iter().zip(peak) {
                    a.fetch_max(p.to_bits(), Ordering::Relaxed);
                }
            },
            |_| {},
            None,
        )
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    #[test]
    fn resample_keeps_length_ratio_and_values() {
        let s = [0.0, 1.0, 1.0, 0.0, 2.0, 1.0, 3.0, 0.0];
        assert_eq!(super::resample(&s, 48000, 48000), s.to_vec());
        let up = super::resample(&s, 24000, 48000);
        assert_eq!(up.len(), 16);
        assert_eq!(&up[..4], &[0.0, 1.0, 0.5, 0.5]);
    }
}
