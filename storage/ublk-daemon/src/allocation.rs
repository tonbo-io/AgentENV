//! Per-acquisition identities. Device numbers and shared image keys are reusable.
use anyhow::{bail, Result};
use std::collections::HashSet;
use uuid::Uuid;

#[derive(Default)]
pub(crate) struct Allocations(HashSet<Uuid>);
impl Allocations {
    pub fn acquire(&mut self) -> Uuid {
        let id = Uuid::now_v7();
        assert!(self.0.insert(id));
        id
    }
    pub fn release(&mut self, id: Uuid) -> Result<bool> {
        if !self.0.remove(&id) {
            bail!("unknown or already released allocation {id}");
        }
        Ok(self.0.is_empty())
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn duplicate_shared_release_cannot_consume_another_reference() {
        let mut refs = Allocations::default();
        let a = refs.acquire();
        let b = refs.acquire();
        assert_ne!(a, b);
        assert!(!refs.release(a).unwrap());
        assert!(refs.release(a).is_err());
        assert_eq!(refs.len(), 1);
        assert!(refs.release(b).unwrap());
    }
    #[test]
    fn stale_release_cannot_consume_a_reused_device_allocation() {
        let mut refs = Allocations::default();
        let old = refs.acquire();
        assert!(refs.release(old).unwrap());
        let current = refs.acquire();
        assert!(refs.release(old).is_err());
        assert!(refs.release(Uuid::nil()).is_err());
        assert_eq!(refs.len(), 1);
        assert!(refs.release(current).unwrap());
    }
    #[test]
    fn only_the_last_distinct_reference_authorizes_device_release() {
        let mut refs = Allocations::default();
        let ids: Vec<_> = (0..8).map(|_| refs.acquire()).collect();
        let refs = std::sync::Arc::new(std::sync::Mutex::new(refs));
        let threads: Vec<_> = ids
            .into_iter()
            .map(|id| {
                let refs = refs.clone();
                std::thread::spawn(move || {
                    let last = refs.lock().unwrap().release(id).unwrap();
                    assert!(refs.lock().unwrap().release(id).is_err());
                    last
                })
            })
            .collect();
        let last_count = threads
            .into_iter()
            .map(|thread| usize::from(thread.join().unwrap()))
            .sum::<usize>();
        assert_eq!(last_count, 1);
        assert_eq!(refs.lock().unwrap().len(), 0);
    }
}
