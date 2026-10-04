//! Managed-layer reference extraction.
//!
//! This module is the one implementation of "which managed layers does this
//! object name" shared by managed-layer GC, node leases, runtime resolution
//! and publication. It recognizes:
//!
//! - the managed-layer key grammar (`managed-layers/sha256:<64 lowercase hex>`);
//! - every `sha256:<64 hex>` token in raw bytes, so records written by a newer
//!   release (unknown fields) or records that fail to parse still protect
//!   their layers;
//! - structured digests of a committed snapshot record;
//! - lower `digest` / `targetDigest` values of an overlaybd `image.json`.

use std::collections::BTreeSet;
use std::path::Path;

use anyhow::{Context, Result};
use overlaybd::config::ImageConfig;

use super::layout::MANAGED_LAYERS_PREFIX;
use crate::snapshot::{
    CommittedAttachedDrive, CommittedSnapshot, OverlaybdLayerRef, SnapshotRecord,
};

const SHA256_PREFIX: &[u8] = b"sha256:";
const SHA256_HEX_LEN: usize = 64;
const SHA256_DIGEST_LEN: usize = SHA256_PREFIX.len() + SHA256_HEX_LEN;

fn is_lower_hex(byte: u8) -> bool {
    byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)
}

/// Whether `value` is a digest the GC may treat as a managed-layer identity.
pub(crate) fn is_managed_digest(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == SHA256_DIGEST_LEN
        && bytes.starts_with(SHA256_PREFIX)
        && bytes[SHA256_PREFIX.len()..]
            .iter()
            .copied()
            .all(is_lower_hex)
}

/// Digest named by a repository-relative managed-layer key. Keys outside the
/// grammar return `None` and are never GC candidates.
pub(crate) fn managed_key_digest(key: &str) -> Option<&str> {
    let digest = key.strip_prefix(MANAGED_LAYERS_PREFIX)?;
    is_managed_digest(digest).then_some(digest)
}

/// Every `sha256:<64 lowercase hex>` token in `bytes`.
pub(crate) fn raw_digest_tokens(bytes: &[u8]) -> BTreeSet<String> {
    let mut tokens = BTreeSet::new();
    let mut index = 0;
    while index + SHA256_DIGEST_LEN <= bytes.len() {
        let candidate = &bytes[index..index + SHA256_DIGEST_LEN];
        if candidate.starts_with(SHA256_PREFIX)
            && candidate[SHA256_PREFIX.len()..]
                .iter()
                .copied()
                .all(is_lower_hex)
        {
            tokens.insert(String::from_utf8_lossy(candidate).into_owned());
            index += SHA256_DIGEST_LEN;
        } else {
            index += 1;
        }
    }
    tokens
}

fn layer_ref_digest(layer: &OverlaybdLayerRef) -> &str {
    match layer {
        OverlaybdLayerRef::Managed(managed) => &managed.digest,
        OverlaybdLayerRef::External(external) => &external.digest,
    }
}

/// Every layer digest a committed snapshot names: rootfs layers (managed and
/// external, whatever their repoBlobUrl), attached-drive layers, memory layers
/// and the tools drive.
pub(crate) fn committed_layer_digests(committed: &CommittedSnapshot) -> BTreeSet<String> {
    let mut digests = BTreeSet::new();
    digests.extend(
        committed
            .rootfs_layers
            .iter()
            .map(|layer| layer_ref_digest(layer).to_string()),
    );
    for drive in &committed.attached_drives {
        match drive {
            CommittedAttachedDrive::Overlaybd { layers, .. } => {
                digests.extend(
                    layers
                        .iter()
                        .map(|layer| layer_ref_digest(layer).to_string()),
                );
            }
        }
    }
    digests.extend(
        committed
            .memory_layers
            .iter()
            .map(|layer| layer.digest.clone()),
    );
    if let Some(tools_drive) = &committed.tools_drive {
        digests.insert(tools_drive.digest.clone());
    }
    digests
}

/// Whether two repoBlobUrls name the same blob location (trailing slashes
/// ignored; an empty URL matches nothing).
pub(crate) fn same_repo_blob_url(left: &str, right: &str) -> bool {
    !left.is_empty() && left.trim_end_matches('/') == right.trim_end_matches('/')
}

/// Digest of a layer stored under this repository's `managed-layers/`:
/// managed refs, and external refs whose repoBlobUrl is this repository's
/// managed-layers URL (snapshots of restored guests record their managed
/// rootfs lowers that way).
pub(crate) fn managed_layer_ref_digest<'a>(
    layer: &'a OverlaybdLayerRef,
    managed_layers_repo_blob_url: &str,
) -> Option<&'a str> {
    match layer {
        OverlaybdLayerRef::Managed(managed) => Some(&managed.digest),
        OverlaybdLayerRef::External(external)
            if is_managed_digest(&external.digest)
                && same_repo_blob_url(&external.repo_blob_url, managed_layers_repo_blob_url) =>
        {
            Some(&external.digest)
        }
        OverlaybdLayerRef::External(_) => None,
    }
}

/// Digests a committed snapshot stores under this repository's
/// `managed-layers/`: managed and managed-URL external rootfs and drive
/// layers, memory layers and the tools drive. These are the objects restore
/// and publication must lease and check.
pub(crate) fn committed_managed_layer_digests(
    committed: &CommittedSnapshot,
    managed_layers_repo_blob_url: &str,
) -> BTreeSet<String> {
    let mut digests = BTreeSet::new();
    let mut add_refs = |layers: &[OverlaybdLayerRef]| {
        digests.extend(layers.iter().filter_map(|layer| {
            managed_layer_ref_digest(layer, managed_layers_repo_blob_url).map(str::to_string)
        }));
    };
    add_refs(&committed.rootfs_layers);
    for drive in &committed.attached_drives {
        match drive {
            CommittedAttachedDrive::Overlaybd { layers, .. } => add_refs(layers),
        }
    }
    digests.extend(
        committed
            .memory_layers
            .iter()
            .map(|layer| layer.digest.clone()),
    );
    if let Some(tools_drive) = &committed.tools_drive {
        digests.insert(tools_drive.digest.clone());
    }
    digests
}

/// Managed-layer digests one catalog record object protects.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct RecordDigests {
    /// Raw token scan united with the structured extraction.
    pub(crate) digests: BTreeSet<String>,
    /// Whether the bytes parsed as a [`SnapshotRecord`].
    pub(crate) parsed: bool,
    /// Whether the parsed record carries a committed payload.
    pub(crate) committed: bool,
}

/// Extract the digests a catalog record object protects.
pub(crate) fn record_digests(bytes: &[u8]) -> RecordDigests {
    let mut digests = raw_digest_tokens(bytes);
    match serde_json::from_slice::<SnapshotRecord>(bytes) {
        Ok(record) => {
            let committed = record.committed.is_some();
            if let Some(payload) = record.committed.as_ref() {
                digests.extend(
                    committed_layer_digests(payload)
                        .into_iter()
                        .filter(|digest| is_managed_digest(digest)),
                );
            }
            RecordDigests {
                digests,
                parsed: true,
                committed,
            }
        }
        Err(_) => RecordDigests {
            digests,
            parsed: false,
            committed: false,
        },
    }
}

/// Managed-layer digests an overlaybd `image.json` reads: every lower's
/// `digest` and `targetDigest` (rootfs, drive and memory configs alike,
/// including `dir=` and `file=` lowers), united with a raw token scan.
pub(crate) fn image_config_digests(path: &Path) -> Result<BTreeSet<String>> {
    let bytes = std::fs::read(path)
        .with_context(|| format!("read overlaybd image config '{}'", path.display()))?;
    let mut digests = raw_digest_tokens(&bytes);
    if let Ok(config) = serde_json::from_slice::<ImageConfig>(&bytes) {
        for layer in config.lowers {
            for digest in [layer.digest, layer.target_digest] {
                if is_managed_digest(&digest) {
                    digests.insert(digest);
                }
            }
        }
    }
    Ok(digests)
}

/// Digests an overlaybd `image.json` reads remotely from this repository's
/// `managed-layers/`: lowers without a local `file` whose effective
/// repoBlobUrl is `managed_layers_repo_blob_url`. These are the objects that
/// must exist before a paused sandbox using the config resumes; local lowers
/// (`file=`) and other registries are not checked.
pub(crate) fn image_config_managed_layer_digests(
    path: &Path,
    managed_layers_repo_blob_url: &str,
) -> Result<BTreeSet<String>> {
    let bytes = std::fs::read(path)
        .with_context(|| format!("read overlaybd image config '{}'", path.display()))?;
    let config = serde_json::from_slice::<ImageConfig>(&bytes)
        .with_context(|| format!("parse overlaybd image config '{}'", path.display()))?;
    Ok(config
        .lowers
        .iter()
        .filter(|layer| {
            layer.file.is_empty()
                && is_managed_digest(&layer.digest)
                && same_repo_blob_url(
                    layer.effective_repo_blob_url(&config.repo_blob_url),
                    managed_layers_repo_blob_url,
                )
        })
        .map(|layer| layer.digest.clone())
        .collect())
}

#[cfg(test)]
pub(crate) fn test_digest(index: usize) -> String {
    format!("sha256:{index:064x}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snapshot::{ExternalLayer, ManagedLayer};
    use serde_json::json;

    fn managed(index: usize) -> OverlaybdLayerRef {
        OverlaybdLayerRef::Managed(ManagedLayer {
            digest: test_digest(index),
            size: 4096,
            uuid: None,
        })
    }

    fn external(index: usize, repo_blob_url: &str) -> OverlaybdLayerRef {
        OverlaybdLayerRef::External(ExternalLayer {
            digest: test_digest(index),
            repo_blob_url: repo_blob_url.to_string(),
            size: 4096,
        })
    }

    const MANAGED_URL: &str = "s3://bucket/prefix/managed-layers";

    fn committed_with_every_layer_kind() -> CommittedSnapshot {
        let mut committed = CommittedSnapshot::mock();
        committed.rootfs_layers = vec![
            managed(1),
            external(2, &format!("{MANAGED_URL}/")),
            external(3, "https://registry.example/v2/ns/image/blobs"),
        ];
        committed.attached_drives = vec![CommittedAttachedDrive::Overlaybd {
            drive_id: "data".to_string(),
            layers: vec![managed(4), external(5, MANAGED_URL)],
            read_only: false,
            virtual_size: 1 << 20,
            mount_path: "/mnt/data".into(),
            sub_path: None,
        }];
        committed.memory_layers = vec![ManagedLayer {
            digest: test_digest(6),
            size: 8192,
            uuid: None,
        }];
        committed.tools_drive = Some(ManagedLayer {
            digest: test_digest(7),
            size: 1024,
            uuid: None,
        });
        committed
    }

    #[test]
    fn raw_scan_finds_sha256_tokens_in_unknown_and_future_record_fields() {
        let record = SnapshotRecord::mock_ready(CommittedSnapshot::mock());
        let mut value = serde_json::to_value(&record).expect("serialize record");
        value["future_field"] = json!({
            "nested": { "layers": [ { "blob": test_digest(42) } ] }
        });
        let bytes = serde_json::to_vec(&value).expect("serialize extended record");

        let extracted = record_digests(&bytes);
        assert!(extracted.digests.contains(&test_digest(42)));

        let unparseable = format!("{{ not json {} ", test_digest(43));
        let extracted = record_digests(unparseable.as_bytes());
        assert!(!extracted.parsed);
        assert!(extracted.digests.contains(&test_digest(43)));

        let tokens = raw_digest_tokens(
            format!(
                "sha256:{} sha256:abc sha256:{}",
                "0".repeat(63),
                "f".repeat(64)
            )
            .as_bytes(),
        );
        assert_eq!(
            tokens,
            BTreeSet::from([format!("sha256:{}", "f".repeat(64))])
        );
    }

    #[test]
    fn structured_and_raw_extraction_cover_every_layer_kind() {
        let committed = committed_with_every_layer_kind();
        let structured = committed_layer_digests(&committed);
        let expected = (1..=7).map(test_digest).collect::<BTreeSet<_>>();
        assert_eq!(structured, expected);

        let managed = committed_managed_layer_digests(&committed, MANAGED_URL);
        assert_eq!(
            managed,
            [1, 2, 4, 5, 6, 7]
                .into_iter()
                .map(test_digest)
                .collect::<BTreeSet<_>>()
        );

        let record = SnapshotRecord::mock_ready(committed);
        let bytes = serde_json::to_vec_pretty(&record).expect("serialize record");
        let extracted = record_digests(&bytes);
        assert!(extracted.parsed);
        assert!(extracted.committed);
        assert!(extracted.digests.is_superset(&expected));
        assert!(raw_digest_tokens(&bytes).is_superset(&expected));
    }

    #[test]
    fn managed_key_grammar_accepts_only_sha256_64_hex() {
        let digest = test_digest(9);
        assert_eq!(
            managed_key_digest(&format!("managed-layers/{digest}")),
            Some(digest.as_str())
        );
        for key in [
            "managed-layers/sha256:abc".to_string(),
            format!("managed-layers/sha256:{}", "A".repeat(64)),
            format!("managed-layers/sha512:{}", "a".repeat(64)),
            format!("managed-layers/{digest}.tmp"),
            format!("managed-layers/nested/{digest}"),
            format!("other/{digest}"),
            "managed-layers/".to_string(),
        ] {
            assert_eq!(managed_key_digest(&key), None, "{key}");
        }
    }

    #[test]
    fn image_config_digests_reads_lowers_digest_and_target_digest() {
        let temp = tempfile::tempdir().expect("tempdir");
        for (name, lowers) in [
            (
                "rootfs.json",
                json!([
                    { "digest": test_digest(1), "size": 1, "dir": "/cache/1" },
                    { "file": "/local/layer.commit", "digest": test_digest(2), "size": 2 }
                ]),
            ),
            (
                "drive.json",
                json!([{ "targetDigest": test_digest(3), "digest": "sha256:short", "size": 3 }]),
            ),
            (
                "memory.json",
                json!([
                    { "digest": test_digest(4), "size": 4, "dir": "/cache/4" },
                    { "file": "/local/mem.commit", "size": 5 }
                ]),
            ),
        ] {
            let path = temp.path().join(name);
            std::fs::write(
                &path,
                serde_json::to_vec(&json!({
                    "repoBlobUrl": MANAGED_URL,
                    "lowers": lowers,
                    "upper": {},
                    "resultFile": ""
                }))
                .expect("serialize image config"),
            )
            .expect("write image config");
            let digests = image_config_digests(&path).expect("read image config digests");
            let expected = match name {
                "rootfs.json" => BTreeSet::from([test_digest(1), test_digest(2)]),
                "drive.json" => BTreeSet::from([test_digest(3)]),
                _ => BTreeSet::from([test_digest(4)]),
            };
            assert_eq!(digests, expected, "{name}");
        }
        assert!(image_config_digests(&temp.path().join("missing.json")).is_err());
    }

    #[test]
    fn image_config_managed_layer_digests_keeps_only_remote_managed_lowers() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("image.json");
        std::fs::write(
            &path,
            serde_json::to_vec(&json!({
                "repoBlobUrl": MANAGED_URL,
                "lowers": [
                    // Remote through the image-level managed URL.
                    { "digest": test_digest(1), "size": 1, "dir": "/cache/1" },
                    // Remote through a per-layer managed URL (trailing slash).
                    { "digest": test_digest(2), "size": 2, "repoBlobUrl": format!("{MANAGED_URL}/") },
                    // Local commit: never checked remotely.
                    { "file": "/local/layer.commit", "digest": test_digest(3), "size": 3 },
                    // Another registry.
                    { "digest": test_digest(4), "size": 4, "repoBlobUrl": "https://registry.example/v2/repo/blobs" },
                    // Not a managed digest.
                    { "digest": "sha256:short", "size": 5 }
                ],
                "upper": {},
                "resultFile": ""
            }))
            .expect("serialize image config"),
        )
        .expect("write image config");
        assert_eq!(
            image_config_managed_layer_digests(&path, MANAGED_URL).expect("managed digests"),
            BTreeSet::from([test_digest(1), test_digest(2)])
        );
        assert!(
            image_config_managed_layer_digests(&temp.path().join("missing.json"), MANAGED_URL)
                .is_err()
        );
    }
}
