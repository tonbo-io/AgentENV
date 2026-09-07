//! In-process creation exclusion. Durable funded activation ownership belongs
//! to sandbox admission; this gate only protects orchestration and cleanup.
use std::collections::HashSet;
use std::io;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Clone, Default)]
pub(super) struct CreationGate(Arc<Mutex<HashSet<Uuid>>>);

pub(super) struct CreationPermit {
    gate: CreationGate,
    id: Uuid,
}

impl CreationGate {
    pub fn try_enter(&self, id: Uuid) -> io::Result<Option<CreationPermit>> {
        let mut active = self
            .0
            .lock()
            .map_err(|_| io::Error::other("creation gate poisoned"))?;
        if !active.insert(id) {
            return Ok(None);
        }
        Ok(Some(CreationPermit {
            gate: self.clone(),
            id,
        }))
    }
}

impl Drop for CreationPermit {
    fn drop(&mut self) {
        // A poisoned gate remains closed to all future entrants.
        if let Ok(mut active) = self.gate.0.lock() {
            active.remove(&self.id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn concurrent_creations_have_one_owner() {
        let gate = CreationGate::default();
        let id = Uuid::now_v7();
        let threads: Vec<_> = (0..16)
            .map(|_| {
                let gate = gate.clone();
                std::thread::spawn(move || gate.try_enter(id).unwrap())
            })
            .collect();
        let permits: Vec<_> = threads
            .into_iter()
            .filter_map(|t| t.join().unwrap())
            .collect();
        assert_eq!(permits.len(), 1);
        assert!(gate.try_enter(id).unwrap().is_none());
        drop(permits);
        assert!(gate.try_enter(id).unwrap().is_some());
    }

    #[test]
    fn unrelated_creation_proceeds_and_unwind_releases_ownership() {
        let gate = CreationGate::default();
        let id = Uuid::now_v7();
        let failed = std::panic::catch_unwind(|| {
            let _permit = gate.try_enter(id).unwrap().unwrap();
            assert!(gate.try_enter(Uuid::now_v7()).unwrap().is_some());
            panic!("injected create failure");
        });
        assert!(failed.is_err());
        assert!(gate.try_enter(id).unwrap().is_some());
    }
}
