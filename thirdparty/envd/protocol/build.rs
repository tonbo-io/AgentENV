fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = prost_build::Config::new();
    config.protoc_executable(protoc_bin_vendored::protoc_bin_path()?);
    config.protoc_arg(format!(
        "-I{}",
        protoc_bin_vendored::include_path()?.display()
    ));
    tonic_prost_build::configure()
        // Needed for older protoc versions used on some Linux hosts.
        .protoc_arg("--experimental_allow_proto3_optional")
        .emit_rerun_if_changed(true)
        .build_server(false)
        .build_client(true)
        .build_transport(false)
        .compile_with_config(
            config,
            &[
                "../proto/filesystem.proto",
                "../../../tools-image/envd-overlay/spec/process/process.proto",
            ],
            &["../proto", "../../../tools-image/envd-overlay/spec/process"],
        )?;

    Ok(())
}
