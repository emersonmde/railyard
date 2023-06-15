fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .compile(
            &["src/railyard/railyard.proto"],
            &["src/railyard/"],
        )?;
    Ok(())
}