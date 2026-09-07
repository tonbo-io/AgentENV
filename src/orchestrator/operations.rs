//! Execution-task accounting, including tasks whose callers stopped waiting.
//! These observations do not replace runtime inventory or recovery fencing.
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OperationStatus {
    pub in_flight: u64,
    pub interrupted: u64,
}

#[derive(Clone, Default)]
pub(super) struct OperationTracker(Arc<Mutex<OperationStatus>>);

pub(super) struct OperationPermit {
    tracker: OperationTracker,
    completed: bool,
}

impl OperationTracker {
    pub fn begin(&self) -> OperationPermit {
        let mut state = self.0.lock().expect("operation tracker poisoned");
        state.in_flight = state.in_flight.checked_add(1).expect("operation overflow");
        OperationPermit {
            tracker: self.clone(),
            completed: false,
        }
    }

    pub fn status(&self) -> OperationStatus {
        *self.0.lock().expect("operation tracker poisoned")
    }
}

impl OperationPermit {
    pub fn complete(mut self) {
        self.completed = true;
    }
}

impl Drop for OperationPermit {
    fn drop(&mut self) {
        let mut state = self.tracker.0.lock().expect("operation tracker poisoned");
        state.in_flight = state.in_flight.checked_sub(1).expect("operation underflow");
        if !self.completed {
            state.interrupted = state
                .interrupted
                .checked_add(1)
                .expect("interruption overflow");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn executor_completion_releases_count_without_erasing_interruption() {
        let tracker = OperationTracker::default();
        let interrupted = tracker.begin();
        let successful = tracker.begin();
        assert_eq!(tracker.status().in_flight, 2);
        drop(interrupted);
        successful.complete();
        assert_eq!(
            tracker.status(),
            OperationStatus {
                in_flight: 0,
                interrupted: 1
            }
        );
    }

    #[test]
    fn panic_does_not_turn_unknown_execution_into_clean_zero() {
        let tracker = OperationTracker::default();
        let executor = tracker.clone();
        assert!(std::thread::spawn(move || {
            let _permit = executor.begin();
            panic!("injected executor failure");
        })
        .join()
        .is_err());
        assert_eq!(
            tracker.status(),
            OperationStatus {
                in_flight: 0,
                interrupted: 1
            }
        );
    }
}
