# TeraPlot

[![CI Status](https://github.com/teraplot/teraplot/workflows/CI/badge.svg)](https://github.com/teraplot/teraplot/actions)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](#license)

> **TeraPlot** is an open-source, ultra-fast time-series visualizer engineered for terabyte-scale datasets.

## Key Features

- **Zero-RAM Overhead:** Uses memory-mapped files (`mmap`) to open multi-gigabyte files instantly (<10ms).
- **GPU-Accelerated:** Renders at 144+ FPS powered by `wgpu` (Vulkan, Metal, Direct3D 12).
- **Sub-millisecond Latency:** Instant zoom and pan via vertex-shader matrix transformations.
- **Speculative Pre-fetching:** Predictive LRU caching for smooth, zero-stutter navigation.
- **Cross-Platform:** Native support for Linux, macOS, and Windows.

## Workspace Architecture

TeraPlot is structured as a Rust Workspace (Edition 2024):

* `crates/teraplot-core`: Headless data processing engine, LOD pyramid generation, and SIMD kernels.
* `crates/teraplot-render`: Low-level `wgpu` rendering pipelines, camera uniform buffers, and WGSL line shaders.
* `crates/teraplot-cli`: Cross-platform desktop application built with `egui`.

## Quick Start

```bash
# Build workspace
cargo build --workspace

# Run CLI app
cargo run -p teraplot-cli -- data.arrow
```

## License

Licensed under either of:

* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT License ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.