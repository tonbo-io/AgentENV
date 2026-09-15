//! Generated envd wire definitions shared by transport adapters.
//! The process schema is authoritative in tools-image/envd-overlay/spec.

pub mod process {
    include!(concat!(env!("OUT_DIR"), "/process.rs"));
}

pub mod filesystem {
    include!(concat!(env!("OUT_DIR"), "/filesystem.rs"));
}
