//! Motore dei job di photo-cat.

pub mod control;
pub mod manager;
pub mod persist;
pub mod pipeline;
pub mod progress;

pub use control::JobControl;
pub use manager::{JobContext, JobFactory, JobManager, JobPlan, JobSnapshot, ProgressSink};
pub use persist::JobState;
pub use pipeline::{Emitter, ItemState, Journal, Pipeline, RunSummary, StageCtx, StageOutcome};
pub use progress::{JobProgress, PhaseCounters, PhaseSnapshot, PhaseSpec, PhaseState};
pub mod bulk;
