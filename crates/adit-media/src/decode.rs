//! Exact frame access: a few persistent sequential readers per file, plus a byte-bounded LRU.

use std::collections::{BTreeMap, HashMap};
use std::ffi::OsStr;
use std::io::Read;
use std::process::ChildStdout;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, sync_channel};
use std::sync::{Arc, Mutex};

use crate::probe::Info;
use crate::proc::{Proc, command, input, run};
use crate::{Frame, MediaError, lock, time_to_frame};

const CACHE_BYTES: usize = 512 << 20;
/// Seek this far before the wanted frame: demuxers seek by dts to a keyframe, and with B-frames
/// that keyframe's pts can lie after the target. The `select` filter drops the surplus.
const SEEK_MARGIN_S: f64 = 1.0;
/// Readers kept open per file (M-FRAME-6): the playing clip plus clips prefetched ahead of cuts.
const MAX_READERS: usize = 4;

type Key = (String, i64);

pub(crate) struct Frames {
    cache: Mutex<Lru>,
    files: Mutex<HashMap<String, Arc<Mutex<FileDec>>>>,
}

struct FileDec {
    // shortcut: up to MAX_READERS idle ffmpegs per video file touched, never reaped; cap the
    // total when projects use many files at once.
    readers: Vec<Arc<Slot>>,
    /// Highest frame index that may exist (exact once a reader has hit the end).
    bound: i64,
    /// Use counter for least-recently-used eviction of readers.
    tick: u64,
}

/// One reader. `next` mirrors `Reader::next` so a reader can be chosen without waiting for the
/// thread that is reading from it; `i64::MAX` = closed.
struct Slot {
    next: AtomicI64,
    used: AtomicU64,
    reader: Mutex<Option<Reader>>,
}

impl Frames {
    pub fn new() -> Self {
        Frames {
            cache: Mutex::new(Lru::new(CACHE_BYTES)),
            files: Mutex::new(HashMap::new()),
        }
    }

    /// Frame `n` (already >= 0) of the file, clamped to the last frame.
    pub fn get(
        &self,
        ffmpeg: &OsStr,
        path: &str,
        info: &Info,
        n: i64,
    ) -> Result<Arc<Frame>, MediaError> {
        let a = self.fetch(ffmpeg, path, info, n, false)?;
        Ok(a.expect("fetch returns a frame unless prefetching"))
    }

    /// M-FRAME-5: decodes frame `n` into the cache on a new reader unless it is cached or an open
    /// reader reaches it. Best effort: errors are dropped.
    pub fn prefetch(&self, ffmpeg: &OsStr, path: &str, info: &Info, n: i64) {
        if !info.still {
            let _ = self.fetch(ffmpeg, path, info, n, true);
        }
    }

    /// `prefetch`: never takes over an existing reader, and returns `None`.
    fn fetch(
        &self,
        ffmpeg: &OsStr,
        path: &str,
        info: &Info,
        n: i64,
        prefetch: bool,
    ) -> Result<Option<Arc<Frame>>, MediaError> {
        let v = info.spec.video.expect("caller checked video");
        let file = lock(&self.files)
            .entry(path.to_string())
            .or_insert_with(|| {
                let bound = info
                    .video_duration
                    .or(info.spec.duration)
                    .map(|d| time_to_frame(d - 1, v.fps).max(0))
                    .unwrap_or(i64::MAX);
                Arc::new(Mutex::new(FileDec {
                    readers: Vec::new(),
                    bound,
                    tick: 0,
                }))
            })
            .clone();
        let frame = |rgba| {
            Arc::new(Frame {
                width: v.width,
                height: v.height,
                rgba,
            })
        };

        if info.still {
            let key = (path.to_string(), 0);
            if let Some(a) = lock(&self.cache).get(&key) {
                return Ok(Some(a));
            }
            let a = frame(decode_still(ffmpeg, path, info)?);
            lock(&self.cache).insert(key, a.clone());
            return Ok(Some(a));
        }

        let fps_frames = i64::from(v.fps.num.div_ceil(v.fps.den).max(1));
        // Reading up to one second forward is cheaper than a new process plus seek.
        let reaches = |next: i64, n: i64| next <= n && n - next <= fps_frames;
        let mut n = n.min(lock(&file).bound);
        let mut seek_from = None;
        loop {
            let key = (path.to_string(), n);
            if let Some(a) = lock(&self.cache).get(&key) {
                return Ok((!prefetch).then_some(a));
            }
            let open = {
                let mut f = lock(&file);
                f.tick += 1;
                let tick = f.tick;
                let open = f
                    .readers
                    .iter()
                    .find(|s| reaches(s.next.load(Ordering::Acquire), n))
                    .cloned();
                if let Some(s) = &open {
                    s.used.store(tick, Ordering::Relaxed);
                }
                open
            };
            let slot = match open {
                Some(_) if prefetch => return Ok(None),
                Some(s) => s,
                None => {
                    // Spawned with no lock held so other callers keep reading this file meanwhile.
                    let start = seek_from.take().unwrap_or(n);
                    let r = Reader::spawn(ffmpeg, path, info, start)?;
                    let s = Arc::new(Slot {
                        next: AtomicI64::new(start),
                        used: AtomicU64::new(0),
                        reader: Mutex::new(Some(r)),
                    });
                    let evicted = {
                        let mut f = lock(&file);
                        f.tick += 1;
                        s.used.store(f.tick, Ordering::Relaxed);
                        let evicted = (f.readers.len() >= MAX_READERS).then(|| {
                            let oldest = (0..f.readers.len())
                                .min_by_key(|&i| f.readers[i].used.load(Ordering::Relaxed))
                                .expect("readers not empty");
                            f.readers.remove(oldest)
                        });
                        f.readers.push(s.clone());
                        evicted
                    };
                    // Killing its ffmpeg waits for the process: not under the file lock.
                    drop(evicted);
                    s
                }
            };
            let mut g = lock(&slot.reader);
            // Another thread may have read past `n` (or closed it) while this one waited.
            let Some(r) = g.as_mut().filter(|r| reaches(r.next, n)) else {
                continue;
            };
            let read = r.read_to(n);
            slot.next.store(r.next, Ordering::Release);
            match read {
                Ok(Some(rgba)) => {
                    // Cached before the slot is released: a caller waiting on it for this frame
                    // then finds it instead of starting another reader.
                    let a = frame(rgba);
                    lock(&self.cache).insert(key, a.clone());
                    drop(g);
                    return Ok((!prefetch).then_some(a));
                }
                Err(e) => {
                    // Its thread has ended; the next request must start a new reader.
                    g.take();
                    close(&file, &slot, g);
                    return Err(e);
                }
                Ok(None) => {
                    // End of stream before frame n: the file is shorter than its stated duration.
                    let mut r = g.take().expect("reader present");
                    close(&file, &slot, g);
                    r.proc.wait()?;
                    let mut f = lock(&file);
                    // Only ever lowered: another reader may already have found the exact end.
                    if r.next > r.start {
                        f.bound = f.bound.min(r.next - 1);
                        seek_from = None;
                    } else if r.start == 0 {
                        return Err(MediaError::Ffmpeg(format!("no video frames in {path}")));
                    } else {
                        f.bound = f.bound.min(r.start - 1);
                        seek_from = Some((r.start - fps_frames).max(0));
                    }
                    n = n.min(f.bound);
                }
            }
        }
    }
}

/// Marks `slot` closed and drops it from the file's readers. Takes the reader guard so the slot
/// lock is released before the file lock is taken (others take file, then slot).
fn close(file: &Mutex<FileDec>, slot: &Arc<Slot>, g: std::sync::MutexGuard<'_, Option<Reader>>) {
    slot.next.store(i64::MAX, Ordering::Release);
    drop(g);
    lock(file).readers.retain(|s| !Arc::ptr_eq(s, slot));
}

fn scale(info: &Info) -> String {
    let v = info.spec.video.expect("video");
    // Forces the buffer size we read, whatever cropping/rotation the decoder would apply.
    format!("scale={}:{}", v.width, v.height)
}

fn frame_len(info: &Info) -> usize {
    let v = info.spec.video.expect("video");
    v.width as usize * v.height as usize * 4
}

fn decode_still(ffmpeg: &OsStr, path: &str, info: &Info) -> Result<Vec<u8>, MediaError> {
    let mut c = command(ffmpeg);
    c.args(["-v", "error", "-nostdin", "-noautorotate", "-i"])
        .arg(input(path))
        .args(["-map", "0:v:0", "-frames:v", "1", "-vf", &scale(info)])
        .args(["-f", "rawvideo", "-pix_fmt", "rgba", "-"]);
    let (out, _) = run(c)?;
    if out.len() != frame_len(info) {
        return Err(MediaError::Ffmpeg(format!(
            "decoded {} bytes, expected {} for {path}",
            out.len(),
            frame_len(info)
        )));
    }
    Ok(out)
}

/// ffmpeg decoding from frame `start` onwards, one raw RGBA frame after another on stdout.
struct Reader {
    /// Frames a thread has read ahead; the read error (EOF at the end) comes last.
    frames: Receiver<std::io::Result<Vec<u8>>>,
    proc: Proc,
    start: i64,
    /// Index of the next frame on stdout.
    next: i64,
}

impl Reader {
    fn spawn(ffmpeg: &OsStr, path: &str, info: &Info, start: i64) -> Result<Self, MediaError> {
        let fps = info.spec.video.expect("video").fps;
        let frame_s = f64::from(fps.den) / f64::from(fps.num);
        let mut c = command(ffmpeg);
        // -copyts keeps the file's own pts, so `t` in select is comparable with frame n's time.
        c.args(["-v", "error", "-nostdin", "-noautorotate", "-copyts"]);
        let first = info.start + start as f64 * frame_s;
        let seek = first - SEEK_MARGIN_S;
        if start > 0 && seek > 0.0 {
            // Without -seek_timestamp, -ss counts from the file's start time and `first` is
            // already absolute: files with a start offset would seek past their target.
            c.args(["-seek_timestamp", "1", "-noaccurate_seek", "-ss"])
                .arg(format!("{seek:.6}"));
        }
        // The fps filter maps by timestamp: output frame k is the frame on screen at slot
        // `start + k` (the latest one not after it, rounded to the nearest slot, which absorbs
        // mkv's 1 ms pts rounding). Same answer for any `start`, so access order does not
        // matter on variable-frame-rate files (H-8).
        c.arg("-i")
            .arg(input(path))
            .args(["-map", "0:v:0", "-fps_mode", "passthrough", "-vf"])
            .arg(format!(
                "fps=fps={}/{}:start_time={first:.9},{}",
                fps.num,
                fps.den,
                scale(info)
            ))
            .args(["-f", "rawvideo", "-pix_fmt", "rgba", "-"]);
        let mut proc = Proc::spawn(c)?;
        let stdout = proc.child.stdout.take().expect("stdout is piped");
        let len = frame_len(info);
        Ok(Reader {
            frames: read_ahead(stdout, len),
            proc,
            start,
            next: start,
        })
    }

    /// Reads forward to frame `n` (needs `next <= n`); `None` at end of stream.
    fn read_to(&mut self, n: i64) -> Result<Option<Vec<u8>>, MediaError> {
        let mut last = None;
        while self.next <= n {
            match self.frames.recv() {
                Ok(Ok(buf)) => {
                    self.next += 1;
                    last = Some(buf);
                }
                Ok(Err(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
                Ok(Err(e)) => return Err(MediaError::Io(format!("reading frames: {e}"))),
                Err(_) => return Err(MediaError::Io("frame reader already failed".into())),
            }
        }
        last.map(Some)
            .ok_or_else(|| MediaError::Io(format!("frame {n} is behind the reader")))
    }
}

/// Frames of `len` bytes from `out`, read on a thread one ahead, so ffmpeg decodes the next frame
/// while the caller renders this one instead of blocking on a full pipe.
// shortcut: an idle reader holds up to 2 frames outside the LRU cap (~66 MB at 4K) and readers
// are never reaped; count them in the cap if many 4K files are open at once.
fn read_ahead(mut out: ChildStdout, len: usize) -> Receiver<std::io::Result<Vec<u8>>> {
    let (tx, rx) = sync_channel(1);
    std::thread::spawn(move || {
        loop {
            let mut buf = vec![0u8; len];
            let r = out.read_exact(&mut buf).map(|()| buf);
            let end = r.is_err();
            // A dropped reader (seek, file closed) ends the thread; killing ffmpeg ends the read.
            if tx.send(r).is_err() || end {
                return;
            }
        }
    });
    rx
}

/// Least recently used frames, bounded by total pixel bytes (the newest entry always stays).
struct Lru {
    map: HashMap<Key, (Arc<Frame>, u64)>,
    order: BTreeMap<u64, Key>,
    tick: u64,
    bytes: usize,
    cap: usize,
}

impl Lru {
    fn new(cap: usize) -> Self {
        Lru {
            map: HashMap::new(),
            order: BTreeMap::new(),
            tick: 0,
            bytes: 0,
            cap,
        }
    }

    fn get(&mut self, key: &Key) -> Option<Arc<Frame>> {
        let e = self.map.get_mut(key)?;
        self.order.remove(&e.1);
        self.tick += 1;
        e.1 = self.tick;
        self.order.insert(self.tick, key.clone());
        Some(e.0.clone())
    }

    fn insert(&mut self, key: Key, f: Arc<Frame>) {
        if let Some((old, t)) = self.map.remove(&key) {
            self.order.remove(&t);
            self.bytes -= old.rgba.len();
        }
        self.tick += 1;
        self.bytes += f.rgba.len();
        self.order.insert(self.tick, key.clone());
        self.map.insert(key, (f, self.tick));
        while self.bytes > self.cap && self.order.len() > 1 {
            let (_, k) = self.order.pop_first().expect("non-empty");
            let (old, _) = self.map.remove(&k).expect("order and map agree");
            self.bytes -= old.rgba.len();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f(len: usize) -> Arc<Frame> {
        Arc::new(Frame {
            width: 1,
            height: 1,
            rgba: vec![0; len],
        })
    }

    #[test]
    fn lru_evicts_least_recent_by_bytes() {
        let k = |n| ("p".to_string(), n);
        let mut l = Lru::new(10);
        let a = f(4);
        l.insert(k(0), a.clone());
        l.insert(k(1), f(4));
        assert!(Arc::ptr_eq(&l.get(&k(0)).unwrap(), &a)); // 0 now most recent
        l.insert(k(2), f(4)); // 12 > 10: evicts 1
        assert!(l.get(&k(1)).is_none());
        assert!(l.get(&k(0)).is_some() && l.get(&k(2)).is_some());
        assert_eq!(l.bytes, 8);
        l.insert(k(3), f(50)); // oversize: keeps only the newest
        assert_eq!((l.map.len(), l.bytes), (1, 50));
        l.insert(k(3), f(2)); // replace keeps byte count right
        assert_eq!(l.bytes, 2);
    }

    /// mkv has no per-stream duration, so the bound comes from the container (2 s of audio)
    /// while video stops after 10 frames: requests past the video end must find its last frame.
    #[test]
    fn video_shorter_than_container_clamps_to_last_frame() {
        use crate::{FLICKS_PER_SECOND as S, MediaBackend};
        let p = std::env::temp_dir().join(format!("adit-media-short-{}.mkv", std::process::id()));
        let out = std::process::Command::new("ffmpeg")
            .args(["-v", "error", "-y", "-f", "lavfi", "-i"])
            .arg("nullsrc=s=16x16:r=10,geq=r='N*20':g=0:b=0")
            .args(["-f", "lavfi", "-i", "sine=d=2", "-frames:v", "10"])
            .args(["-c:v", "ffv1", "-c:a", "pcm_s16le"])
            .arg(&p)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let path = p.to_str().unwrap();
        let ff = crate::Ffmpeg::new().unwrap();
        for t in [3 * S / 2, S / 2 * 3 + 1, 5 * S, 19 * S / 20] {
            let fr = ff.video_frame(path, t).unwrap();
            assert_eq!(fr.rgba[0], 180, "t={t}");
        }
        assert_eq!(ff.video_frame(path, S / 2).unwrap().rgba[0], 100);
        drop(ff);
        let _ = std::fs::remove_file(&p);
    }

    /// `-ss` counts from the file's start time: a seek computed from absolute pts once landed
    /// `start_time` too late. Only visible when a keyframe exists near that wrong target, so the
    /// file is intra-only (ffv1) and long enough; one-GOP files fall back to frame 0 and pass.
    #[test]
    fn cold_seek_on_offset_file_is_not_offset_twice() {
        use crate::{FLICKS_PER_SECOND as S, MediaBackend};
        let p = std::env::temp_dir().join(format!("adit-media-ofs-{}.mkv", std::process::id()));
        let out = std::process::Command::new("ffmpeg")
            .args(["-v", "error", "-y", "-f", "lavfi", "-i"])
            .arg("nullsrc=s=16x16:r=10,geq=r='N*4':g=0:b=0")
            .args(["-frames:v", "60", "-c:v", "ffv1", "-output_ts_offset", "10"])
            .arg(&p)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let path = p.to_str().unwrap();
        for n in [25i64, 40, 3] {
            let ff = crate::Ffmpeg::new().unwrap();
            let fr = ff.video_frame(path, n * S / 10).unwrap();
            assert_eq!(i64::from(fr.rgba[0]), n * 4, "cold read of frame {n}");
        }
        let _ = std::fs::remove_file(&p);
    }
}
