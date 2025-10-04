fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting protobuf compilation...");
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .out_dir("src/proto/")
        .compile_protos(
            &["src/proto/raftGrpc.proto"],
            &["src/proto/"]
        )?;
    println!("Protobuf compilation completed.");
    Ok(())
}