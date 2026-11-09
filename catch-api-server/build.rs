// Compile the gRPC face's proto into the tonic server stubs.
//
// Needs `protoc` on PATH at build time (the nix/gen build supplies it via a
// crate override; locally: `nix shell nixpkgs#protobuf`).
//
// Path resolution must work under BOTH build models:
//   - cargo dev: CWD / CARGO_MANIFEST_DIR == the crate dir (catch-api-server/),
//     so `proto/catch.proto` resolves directly.
//   - gen / substrate lockfile-builder: the crate is built with
//     `src = workspaceSrc` (the whole workspace), so CWD / CARGO_MANIFEST_DIR
//     is the WORKSPACE ROOT and the proto lives at
//     `catch-api-server/proto/catch.proto`.
// Probing a small set of candidate roots covers both without a hardcode that
// breaks the other model.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::path::PathBuf;

    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
    let candidates = [
        PathBuf::from(&manifest).join("proto"), // cargo: crate dir
        PathBuf::from(&manifest).join("catch-api-server").join("proto"), // gen: workspace root
        PathBuf::from("proto"),                 // fallback: CWD == crate dir
        PathBuf::from("catch-api-server").join("proto"), // fallback: CWD == workspace root
    ];
    let proto_dir = candidates
        .iter()
        .find(|p| p.join("catch.proto").is_file())
        .ok_or("catch-api-server build.rs: could not locate proto/catch.proto")?;
    let proto_file = proto_dir.join("catch.proto");

    println!("cargo:rerun-if-changed={}", proto_file.display());

    tonic_build::configure()
        .build_client(false)
        .compile_protos(&[proto_file], &[proto_dir.clone()])?;
    Ok(())
}
