pub fn generate() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = "src/event_store/generated/";
    let files = [
        "protos/persistent.proto",
        "protos/streams.proto",
        "protos/shared.proto",
        "protos/gossip.proto",
    ];

    tonic_build::configure()
        .build_server(false)
        .out_dir(out_dir)
        .compile(&files, &["protos"])?;

    let gen_dir = std::fs::read_dir(out_dir)?;

    for entry in gen_dir {
        let file = entry.unwrap();
        let filename_string = file.file_name().into_string().expect("valid utf-8 string");
        if filename_string.starts_with("event_store.client.") {
            let remaining = filename_string.trim_start_matches("event_store.client.");
            let new_file_name = if remaining == "persistent_subscriptions.rs" {
                "persistent.rs"
            } else if filename_string.as_str() == "event_store.client.rs" {
                "client.rs"
            } else {
                remaining
            };

            let new_file = file.path().parent().unwrap().join(new_file_name);
            std::fs::rename(file.path(), new_file)?;
        } else if filename_string.as_str() == "google.rpc.rs" {
            let new_file = file.path().parent().unwrap().join("google_rpc.rs");
            std::fs::rename(file.path(), new_file)?;
        }
    }

    Ok(())
}

fn main() {
    generate().unwrap_or_else(|e| panic!("Failed to compile protos {:?}", e));
}
