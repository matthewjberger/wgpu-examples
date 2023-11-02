# WGPU Examples

This repo contains examples of how to use wgpu and egui to render 3D scenes in Rust 🚀

## Crates

* [wgpu](https://github.com/gfx-rs/wgpu) is a rust rendering API that abstracts multiple graphics backends into a unified API.
* [egui](https://github.com/emilk/egui) is an immediate mode gui library written in rust.


## Quickstart

```
# Clone the repo
git clone https://github.com/matthewjberger/wgpu-examples
cd ./wgpu-examples

# View apps
ls src/bin/*

# Run apps
# Setting the RUST_LOG env var to `info` here enables logging
RUST_LOG=info cargo run -r --bin triangle
```
