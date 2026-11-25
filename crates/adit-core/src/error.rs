use crate::Id;

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum Error {
    #[error("{what} {id} not found")]
    NotFound { what: &'static str, id: Id },
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
    #[error("clips would overlap on track {track_id}")]
    Overlap { track_id: Id },
    #[error("track {track_id} is locked")]
    TrackLocked { track_id: Id },
    #[error("ripple blocked on track {track_id}")]
    RippleBlocked { track_id: Id },
    #[error("clip {clip_id} would read outside its source")]
    SourceOutOfRange { clip_id: Id },
    #[error("clip source does not fit: {0}")]
    KindMismatch(String),
    #[error("{what} {id} is still in use")]
    InUse { what: &'static str, id: Id },
    #[error("clip {clip_id} has no adjacent clip on that edge")]
    NoAdjacentClip { clip_id: Id },
    #[error("sequence {sequence_id} would contain itself")]
    NestCycle { sequence_id: Id },
}
