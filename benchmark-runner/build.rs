fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .out_dir("src")
        .compile(&["../metrics/metrics.proto"], &["../metrics"])?;

    println!("cargo:rerun-if-changed=./migrations");
    Ok(())
}
