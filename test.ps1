pushd ..
cargo insta test --review --all --workspace-root noye --manifest-path .\noye\Cargo.toml
popd
