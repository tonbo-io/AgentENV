//! Node-local admission accounting. Closing admission does not prove runtime
//! cleanup, durable drain intent, or permission to terminate the host.

use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AdmissionStatus {
    pub closed: bool,
    pub in_flight: usize,
}

#[derive(Clone, Default)]
pub(super) struct NodeAdmission(Arc<Mutex<AdmissionStatus>>);

pub(super) struct AdmissionPermit(Arc<Mutex<AdmissionStatus>>);

impl NodeAdmission {
    pub fn acquire(&self) -> Option<AdmissionPermit> {
        let mut state = self.0.lock().expect("node admission poisoned");
        if state.closed {
            return None;
        }
        state.in_flight = state.in_flight.checked_add(1).expect("admission overflow");
        Some(AdmissionPermit(Arc::clone(&self.0)))
    }

    /// Irreversible for this service instance. Existing permits remain counted
    /// until the actual cancellation-safe operation finishes, including rollback.
    pub fn close(&self) -> AdmissionStatus {
        let mut state = self.0.lock().expect("node admission poisoned");
        state.closed = true;
        *state
    }

    pub fn status(&self) -> AdmissionStatus {
        *self.0.lock().expect("node admission poisoned")
    }
}

impl Drop for AdmissionPermit {
    fn drop(&mut self) {
        let mut state = self.0.lock().expect("node admission poisoned");
        state.in_flight = state.in_flight.checked_sub(1).expect("admission underflow");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    #[test]
    fn closure_accounts_for_work_until_executor_releases_it() {
        let gate = NodeAdmission::default();
        let permit = gate.acquire().unwrap();
        let (finish, wait) = mpsc::channel();
        let executor = std::thread::spawn(move || {
            let _permit = permit;
            wait.recv().unwrap();
        });
        assert_eq!(
            gate.close(),
            AdmissionStatus {
                closed: true,
                in_flight: 1
            }
        );
        assert!(gate.acquire().is_none());
        assert_eq!(gate.close().in_flight, 1);
        finish.send(()).unwrap();
        executor.join().unwrap();
        assert_eq!(
            gate.status(),
            AdmissionStatus {
                closed: true,
                in_flight: 0
            }
        );
        assert!(gate.acquire().is_none());
    }

    #[test]
    fn concurrent_admission_cannot_cross_a_completed_close() {
        let gate = NodeAdmission::default();
        let barrier = Arc::new(std::sync::Barrier::new(9));
        let workers: Vec<_> = (0..8)
            .map(|_| {
                let gate = gate.clone();
                let barrier = Arc::clone(&barrier);
                std::thread::spawn(move || {
                    barrier.wait();
                    for _ in 0..1000 {
                        let _permit = gate.acquire();
                        std::thread::yield_now();
                    }
                })
            })
            .collect();
        barrier.wait();
        gate.close();
        for _ in 0..1000 {
            assert!(gate.acquire().is_none());
        }
        for worker in workers {
            worker.join().unwrap();
        }
        assert_eq!(
            gate.status(),
            AdmissionStatus {
                closed: true,
                in_flight: 0
            }
        );
    }
}
