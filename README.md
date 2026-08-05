# PetaPlot

[![CI Status](https://github.com/petaplot/petaplot/workflows/CI/badge.svg)](https://github.com/petaplot/petaplot/actions)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](#license)

> **PetaPlot** is an open-source, ultra-fast time-series visualizer engineered for terabyte-scale datasets.

## Key Features

- **Zero-RAM Overhead:** Uses memory-mapped files (`mmap`) to open multi-gigabyte files instantly (<10ms).
- **GPU-Accelerated:** Renders at 144+ FPS powered by `wgpu` (Vulkan, Metal, Direct3D 12).
- **Sub-millisecond Latency:** Instant zoom and pan via vertex-shader matrix transformations.
- **Speculative Pre-fetching:** Predictive LRU caching for smooth, zero-stutter navigation.
- **Cross-Platform:** Native support for Linux, macOS, and Windows.

## Workspace Architecture

PetaPlot is structured as a Rust Workspace (Edition 2024):

* `crates/petaplot-core`: Headless data processing engine, LOD pyramid generation, and SIMD kernels.
* `crates/petaplot-render`: Low-level `wgpu` rendering pipelines, camera uniform buffers, and WGSL line shaders.
* `crates/petaplot`: Cross-platform desktop application built with `egui`.

## Quick Start

```bash
# Run locally from workspace with synthetic demo
cargo run -p petaplot

# Open a Parquet time-series dataset directly
cargo run -p petaplot -- data.parquet
```

## Benchmark Datasets (GWOSC Gravitational Wave Data)

PetaPlot includes a script to download public 16 kHz strain data from GWOSC directly into a Parquet file:

```bash
uv pip install h5py pyarrow numpy gwosc

# Micro-benchmark (GW150914 - ~67 Million points)
uv run scripts/download_gwosc_parquet.py --preset gw150914 -o gw150914.parquet

# Open with PetaPlot
cargo run -p petaplot --release -- gw150914.parquet
```

## License

Licensed under either of:

* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT License ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.