# Fuzzing the subscription contract

Coverage-guided fuzzing with [cargo-fuzz](https://github.com/rust-fuzz/cargo-fuzz).
The target `entry_points` drives subscribe, charge, cancel, pause and resume
with arbitrary arguments and checks that nothing panics outside a contract
error and that remaining cap never goes negative.

```bash
cargo install cargo-fuzz --locked
rustup toolchain install nightly
cd fuzz
cargo +nightly fuzz run entry_points -- -max_total_time=60
```

A crash is saved under `fuzz/artifacts/entry_points/`; reproduce it with
`cargo +nightly fuzz run entry_points <artifact-path>`. A scheduled workflow
(`.github/workflows/fuzz.yml`) runs this weekly for a fixed time and uploads
any crash as a build artifact.
