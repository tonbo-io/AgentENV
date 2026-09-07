//! Node-local, irreversible runtime creation claims. These are not grants or
//! evidence that a runtime started or stopped. Unknown outcomes stay consumed.
use std::collections::HashSet;
use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Clone, Default)]
pub(super) struct CreationClaims {
    directory: Option<PathBuf>,
    consumed: Arc<Mutex<HashSet<Uuid>>>,
}

impl CreationClaims {
    pub fn persistent(directory: PathBuf) -> io::Result<Self> {
        fs::create_dir_all(&directory)?;
        for ancestor in directory.ancestors() {
            File::open(ancestor)?.sync_all()?;
        }
        Ok(Self {
            directory: Some(directory),
            ..Self::default()
        })
    }

    /// Returns true only to the first claimant. A failed fsync never authorizes
    /// execution and never removes a possibly persisted claim.
    pub fn claim(&self, id: Uuid) -> io::Result<bool> {
        if !self
            .consumed
            .lock()
            .map_err(|_| io::Error::other("creation claims poisoned"))?
            .insert(id)
        {
            return Ok(false);
        }
        if let Some(directory) = &self.directory {
            let file = match OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(directory.join(id.to_string()))
            {
                Ok(file) => file,
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => return Ok(false),
                Err(error) => return Err(error),
            };
            file.sync_all()?;
            File::open(directory)?.sync_all()?;
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn concurrent_claimants_have_one_winner() {
        let claims = CreationClaims::default();
        let id = Uuid::now_v7();
        let threads: Vec<_> = (0..16)
            .map(|_| {
                let claims = claims.clone();
                std::thread::spawn(move || claims.claim(id).unwrap())
            })
            .collect();
        assert_eq!(
            threads
                .into_iter()
                .filter_map(|t| t.join().unwrap().then_some(()))
                .count(),
            1
        );
        assert!(!claims.claim(id).unwrap());
    }

    #[test]
    fn disk_claim_survives_reopen_and_independent_claimants() {
        let temp = tempfile::tempdir().unwrap();
        let directory = temp.path().join("claims");
        let a = CreationClaims::persistent(directory.clone()).unwrap();
        let b = CreationClaims::persistent(directory.clone()).unwrap();
        let id = Uuid::now_v7();
        assert!(a.claim(id).unwrap());
        assert!(!b.claim(id).unwrap());
        drop(a);
        drop(b);
        let restored = CreationClaims::persistent(directory).unwrap();
        assert!(!restored.claim(id).unwrap());
        assert!(restored.claim(Uuid::now_v7()).unwrap());
    }

    #[test]
    fn io_failure_does_not_reopen_an_uncertain_claim() {
        let temp = tempfile::tempdir().unwrap();
        let directory = temp.path().join("claims");
        let claims = CreationClaims::persistent(directory.clone()).unwrap();
        fs::remove_dir(&directory).unwrap();
        let id = Uuid::now_v7();
        assert!(claims.claim(id).is_err());
        fs::create_dir(&directory).unwrap();
        assert!(!claims.claim(id).unwrap());
    }
}
