//! Export (SPEC X-1..X-3): rendered frames and the mixed soundtrack into `adit_media::Encoder`.

use crate::{RenderError, Renderer, audio};
use adit_core::*;
use adit_media::{AudioCodec, Encoder, ExportSettings, MediaBackend, MediaError};
use std::sync::atomic::{AtomicBool, Ordering};

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
    let mut enc = Encoder::start(settings, sound.as_deref())?;
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
        Ok(()) => enc.finish().map_err(RenderError::from),
        Err(e) => {
            drop(enc);
            Err(e)
        }
    };
    if r.is_err() {
        let _ = std::fs::remove_file(&settings.path);
    }
    r
}
