//! Export (SPEC X-1..X-3): rendered frames and the mixed soundtrack into `adit_media::Encoder`.

use crate::{RenderError, Renderer, audio};
use adit_core::*;
use adit_media::{AudioCodec, Encoder, ExportSettings, MediaBackend, MediaError};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

pub(crate) fn export(
    renderer: &mut Renderer,
    project: &Project,
    sequence_id: Id,
    settings: &ExportSettings,
    media: &dyn MediaBackend,
    progress: &mut dyn FnMut(u64, u64),
    cancel: &AtomicBool,
) -> Result<(), RenderError> {
    let seq = project
        .sequence(sequence_id)
        .ok_or(RenderError::SequenceNotFound(sequence_id))?;
    let duration = seq.duration();
    if duration <= 0 {
        return Err(
            MediaError::Unsupported("nothing to export: the sequence is empty".into()).into(),
        );
    }
    // H-5 shortcut: the soundtrack is mixed in memory, so length is capped; stream it to lift this.
    if duration > MAX_EXPORT {
        return Err(MediaError::Unsupported(
            "the sequence is longer than 4 hours, the export limit".into(),
        )
        .into());
    }
    if settings.fps.num == 0 || settings.fps.den == 0 {
        return Err(MediaError::Unsupported(format!("export fps {:?}", settings.fps)).into());
    }
    // X-1: the frames n >= 0 with frame_to_time(n) < duration.
    let total = time_to_frame(duration - 1, settings.fps) + 1;
    if cancel.load(Ordering::SeqCst) {
        return Err(RenderError::Cancelled);
    }
    let has_audio = seq.tracks.iter().any(|t| t.kind == TrackKind::Audio);
    let sound = if settings.audio != AudioCodec::None && has_audio {
        let v = audio::mix_audio(project, sequence_id, 0, duration, media)?;
        Some(audio::resample(&v, seq.sample_rate, settings.sample_rate))
    } else {
        None
    };
    // H-12: encode next to the target and rename over it only on success, so a file already at
    // the target survives a failure or cancel.
    let target = Path::new(&settings.path);
    let tmp = temp_path(target);
    let tmp_settings = ExportSettings {
        path: tmp.to_string_lossy().into_owned(),
        ..settings.clone()
    };
    let mut enc = match Encoder::start(&tmp_settings, sound.as_deref()) {
        Ok(e) => e,
        Err(e) => {
            let _ = std::fs::remove_file(&tmp);
            return Err(e.into());
        }
    };
    let r = (|| {
        for n in 0..total {
            if cancel.load(Ordering::SeqCst) {
                return Err(RenderError::Cancelled);
            }
            let t = frame_to_time(n, settings.fps);
            let rgba =
                renderer.render_sized(project, seq, t, media, (settings.width, settings.height))?;
            enc.write_frame(&rgba)?;
            progress(n as u64 + 1, total as u64);
        }
        Ok(())
    })();
    // Dropping the encoder kills and waits for ffmpeg, so the file is closed before removal.
    let r = match r {
        Ok(()) => enc.finish().map_err(RenderError::from).and_then(|()| {
            std::fs::rename(&tmp, target).map_err(|e| {
                MediaError::Io(format!(
                    "cannot move the export to {}: {e}",
                    target.display()
                ))
                .into()
            })
        }),
        Err(e) => {
            drop(enc);
            Err(e)
        }
    };
    if r.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    r
}

const MAX_EXPORT: Time = 4 * 3600 * FLICKS_PER_SECOND;

/// `dir/name.adit-tmp-<pid>-<n>.ext`: same extension, so ffmpeg picks the same container; unique
/// so concurrent exports to one target do not share it.
fn temp_path(target: &Path) -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let mut name = target.file_stem().unwrap_or_default().to_os_string();
    name.push(format!(
        ".adit-tmp-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    if let Some(ext) = target.extension() {
        name.push(".");
        name.push(ext);
    }
    target.with_file_name(name)
}
