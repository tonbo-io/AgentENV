//! Allocated disk bytes of a sandbox runtime.
//!
//! Every byte a sandbox writes lands in its work directory: the overlaybd
//! upper layer of the rootfs, one runtime directory per extra drive, and the
//! Firecracker logs and socket. The ublk daemon writes those files on the
//! sandbox's behalf, so the Firecracker cgroup's `io.stat` never sees them
//! and the directory is measured instead. Upper layers are sparse, so the
//! measurement is allocated blocks, not file length.

use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

/// Bytes allocated on disk by regular files under `root`, symlinks excluded.
///
/// Files that vanish or cannot be read mid-walk count as zero: a sandbox that
/// is being torn down must not fail the whole sampling pass.
pub(crate) fn allocated_bytes(root: &Path) -> u64 {
    let mut total = 0u64;
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(metadata) = entry.metadata() else {
                continue;
            };
            let file_type = metadata.file_type();
            if file_type.is_symlink() {
                continue;
            }
            if file_type.is_dir() {
                pending.push(entry.path());
            } else if file_type.is_file() {
                total = total.saturating_add(metadata.blocks().saturating_mul(512));
            }
        }
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Seek, SeekFrom, Write};
    use tempfile::tempdir;

    #[test]
    fn counts_allocated_blocks_not_sparse_length() {
        let temp = tempdir().unwrap();
        let nested = temp.path().join("overlaybd");
        fs::create_dir_all(&nested).unwrap();

        let mut sparse = fs::File::create(nested.join("upper.data")).unwrap();
        sparse.set_len(64 * 1024 * 1024).unwrap();
        sparse.seek(SeekFrom::Start(0)).unwrap();
        sparse.write_all(&[7u8; 4096]).unwrap();
        sparse.sync_all().unwrap();

        let allocated = allocated_bytes(temp.path());
        assert!(allocated >= 4096, "written block is allocated: {allocated}");
        assert!(
            allocated < 64 * 1024 * 1024,
            "sparse tail is not allocated: {allocated}"
        );
    }

    #[test]
    fn symlinks_and_missing_roots_do_not_count() {
        let temp = tempdir().unwrap();
        let target = temp.path().join("outside.bin");
        fs::write(&target, vec![1u8; 1 << 20]).unwrap();
        let root = temp.path().join("work");
        fs::create_dir_all(&root).unwrap();
        std::os::unix::fs::symlink(&target, root.join("user-rootfs")).unwrap();

        assert_eq!(allocated_bytes(&root), 0);
        assert_eq!(allocated_bytes(&temp.path().join("absent")), 0);
    }
}
