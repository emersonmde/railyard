fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure().compile(
        &["proto/railyard.proto", "proto/cluster_management.proto"],
        &["proto/"],
    )?;
    Ok(())
}
