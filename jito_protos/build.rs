use tonic_build::configure;

fn main() {
    if std::env::var("PROTOC").is_err() {
        panic!(
            "PROTOC is not set. Install protoc and set PROTOC to the protoc.exe path \
             (Windows cannot use protobuf-src, which requires Unix sh)."
        );
    }

    configure()
        .compile(
            &[
                "protos/shared.proto",
                "protos/auth.proto",
                "protos/shredstream.proto",
                "protos/block.proto",
                "protos/block_engine.proto",
                "protos/bundle.proto",
                "protos/packet.proto",
                "protos/relayer.proto",
                "protos/searcher.proto",
            ],
            &["protos"],
        )
        .unwrap();
}
