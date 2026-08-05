# Plan de Arquitectura e Implementación: PetaPlot

Este documento define la especificación técnica completa y detallada para el desarrollo de **PetaPlot**: un visor de series temporales de rendimiento extremo (*zero-copy*, latencia sub-milisegundo, renderizado a 60-140+ FPS y soporte para datasets masivos *larger-than-RAM*).

El objetivo de este plan es servir como **especificación técnica directa para agentes de programación y desarrolladores**, cubriendo desde la arquitectura del pipeline de datos hasta la estructura de crates en Rust y los shaders de renderizado.

---

## 1. Visión General del Sistema y Métricas Objetivo

PetaPlot aborda el cuello de botella tradicional de las herramientas de visualización (MATLAB, PlotJuggler, scripts en Python): la dependencia de la memoria RAM y el *overhead* de parseo de archivos.

### Métricas de Rendimiento Objetivo (SLAs)

* **Tiempo de apertura de archivo:** $< 10\text{ ms}$ independientemente del tamaño del dataset ($10\text{ MB}$ a $10\text{ TB}$).
* **Latencia de interacción (Pan/Zoom):** $< 1\text{ ms}$ (procesamiento directo en Vertex Shader).
* **Tasa de refresco de renderizado:** 60 - 144+ FPS estables sin tirones (*frame drops*).
* **Consumo de memoria RAM:** Constante ($100\text{ MB} - 300\text{ MB}$), independiente del tamaño del archivo en disco.
* **Compatibilidad Multiplataforma:** Nativamente en Linux (Vulkan), macOS (Metal) y Windows (Direct3D 12).

---

## 2. Stack Tecnológico

| Componente | Tecnología Seleccionada | Justificación |
| --- | --- | --- |
| **Lenguaje Base** | **Rust 2021 Edition** | Garantía de concurrencia segura, control estricto de memoria y optimizaciones SIMD. |
| **I/O y Memoria Virtual** | `memmap2` | Mapeo de memoria virtual en disco (`mmap`) con cero costo de asignación inicial. |
| **Motor Gráfico (Hardware)** | `wgpu` (v23.0+) | Abstracción sobre Vulkan (Linux), Metal (macOS) y Direct3D 12 (Windows). |
| **Interfaz de Usuario (GUI)** | `egui` (v0.29+) | Framework de UI inmediata (*Immediate Mode UI*), ultraligero y desacoplado del pipeline gráfico. |
| **Paralelismo y Computación** | `rayon` + Intrínsicos SIMD (`std::arch`) | Vectorización de la decimación Min-Max por hardware (AVX-2 / AVX-512 / NEON). |
| **Formato en Memoria / IPC** | `arrow` / `arrow-array` | Estructura de datos en columnas (*zero-copy*) sin etapas de parseo intermediate. |
| **Benchmarking** | `criterion` | Pruebas de regresión microsegundo a microsegundo para funciones de E/S y decimación. |

---

## 3. Arquitectura del Sistema: Pipeline de Datos de 3 Capas

```
┌────────────────────────────────────────────────────────────────────────┐
│                          1. CAPA DE ALMACENAMIENTO                     │
│  [ Archivo Binario en Disco / Format Native (Arrow, Parquet, Zarr) ]  │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │
                        `mmap` (Cero-Copia) / Chunks de 4 KB
                                    │
                                    ▼
┌────────────────────────────────────────────────────────────────────────┐
│                       2. CAPA DE DATOS & CACHÉ (CPU)                   │
│  - Pirámide de Niveles de Detalle (LOD Min-Max)                        │
│  - Hilo Predictor de Navegación (Speculative Pre-fetching LRU Cache)   │
│  - Decimador SIMD Paralelo (Rayon)                                     │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │
                       VBO Instanced Buffers / Uniforms
                                    │
                                    ▼
┌────────────────────────────────────────────────────────────────────────┐
│                     3. CAPA DE RENDERIZADO & UI (GPU)                  │
│  - Pipeline wgpu (Compute + Render Passes)                             │
│  - Transformación Matriz-Viewport en Vertex Shader (0ms latencia CPU)  │
│  - Shell UI con egui (Overlay desacoplado)                             │
└────────────────────────────────────────────────────────────────────────┘

```

---

## 4. Detalle Técnico de los Componentes Core

### 4.1. Estructura de la Pirámide de Datos Min-Max (LOD)

Para una serie temporal de $N$ muestras, se genera o lee una pirámide logarítmica de factor $10\times$:

* **LOD 0 (Raw):** Todos los puntos originales $(t_i, y_i)$.
* **LOD 1 (1/10):** 1 par $(y_{\min}, y_{\max})$ por cada 10 muestras de LOD 0.
* **LOD $k$ ($1/10^k$):** Resumen reduciendo iterativamente los pares Min-Max.

*Algoritmo de Selección de Nivel (Viewport Sampling):*


$$\text{LOD}_{\text{objetivo}} = \left\lfloor \log_{10} \left( \frac{N_{\text{muestras\_en\_rango\_visible}}}{W_{\text{píxeles\_pantalla}} \times \text{Factor\_Over-sampling}} \right) \right\rfloor$$

### 4.2. Hilo Predictor de Navegación (Speculative Pre-fetching)

Un hilo asíncrono analiza la velocidad $\vec{v} = \frac{\Delta x}{\Delta t}$ e inercia de interacción del ratón/trackpad:

1. Proyecta las coordenadas de tiempo $T_{\text{futuro}} = T_{\text{actual}} + \vec{v} \cdot \Delta t_{\text{preview}}$.
2. Solicita al kernel mediante `mmap::advise(Advice::WillNeed)` los bloques de memoria virtual correspondientes a los próximos $200\text{ ms}$ de desplazamiento.
3. Almacena el resultado decimado en un **Ring Buffer** de la CPU con estrategia de reemplazo **LRU**.

### 4.3. Renderizado GPU Mediante Dibujo por Instancias (*Instanced Line Strip*)

Para evitar enviar vértices individuales a la GPU en cada frame:

* Se envía a la GPU una **instancia base de una línea** de punto $A(0, y_{\min})$ a punto $B(1, y_{\max})$.
* Se pasa un *Instance Buffer* conteniendo los pares $(y_{\min}, y_{\max})$ de cada bin horizontal.
* El **Vertex Shader** calcula las coordenadas finales en pantalla utilizando la matriz de transformación del *Viewport* (Offset X, Scale X, Offset Y, Scale Y). El zoom y el pan cambian únicamente **16 floats** en un *Uniform Buffer*, sin reescribir geometría.

---

## 5. Estructura Completa del Repositorio (Cargo Workspace)

```text
petaplot/
├── Cargo.toml
├── LICENSE-APACHE
├── LICENSE-MIT
├── README.md
├── rustfmt.toml
├── .github/
│   └── workflows/
│       ├── ci.yml
│       └── release.yml
├── benches/
│   └── simd_decimation.rs
├── crates/
│   ├── petaplot-core/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── error.rs
│   │       ├── storage/
│   │       │   ├── mod.rs
│   │       │   ├── mmap_reader.rs
│   │       │   └── arrow_layout.rs
│   │       ├── lod/
│   │       │   ├── mod.rs
│   │       │   ├── pyramid.rs
│   │       │   └── builder.rs
│   │       └── compute/
│   │           ├── mod.rs
│   │           ├── simd.rs
│   │           └── prefetcher.rs
│   ├── petaplot-render/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── context.rs
│   │       ├── pipeline.rs
│   │       ├── camera.rs
│   │       └── shaders/
│   │           ├── line.wgsl
│   │           └── grid.wgsl
│   └── petaplot-cli/
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs
│           ├── app.rs
│           ├── state.rs
│           └── ui/
│               ├── mod.rs
│               ├── plot_view.rs
│               └── controls.rs
└── docs/
    └── index.md

```

---

## 6. Ficheros de Configuración e Implementación Base

### 6.1. Root `Cargo.toml`

```toml
[workspace]
resolver = "2"
members = [
    "crates/petaplot-core",
    "crates/petaplot-render",
    "crates/petaplot-cli",
]

[workspace.package]
version = "0.1.0"
edition = "2021"
authors = ["Fidel Echevarria <contact@petaplot.dev>"]
license = "MIT OR Apache-2.0"
repository = "https://github.com/petaplot/petaplot"
homepage = "https://petaplot.dev"

[workspace.dependencies]
# Internas
petaplot-core = { path = "crates/petaplot-core" }
petaplot-render = { path = "crates/petaplot-render" }

# Rendimiento e I/O
memmap2 = "0.9"
bytemuck = { version = "1.18", features = ["derive"] }
rayon = "1.10"
arrow-array = "53.0"

# Gráficos y UI
wgpu = "23.0"
egui = "0.29"
egui-wgpu = "0.29"
winit = "0.30"

# Utilidades y Logs
thiserror = "1.0"
tracing = "0.1"
tracing-subscriber = "0.3"

[profile.release]
opt-level = 3
lto = "fat"
codegen-units = 1
panic = "abort"
strip = true

```

### 6.2. Kernel de Decimación SIMD (`crates/petaplot-core/src/compute/simd.rs`)

```rust
pub struct MinMaxPair {
    pub min: f32,
    pub max: f32,
}

/// Decimación en paralelo usando chunking SIMD
pub fn reduce_min_max_chunk(data: &[f32], bin_size: usize) -> Vec<MinMaxPair> {
    use rayon::prelude::*;

    data.par_chunks(bin_size)
        .map(|chunk| {
            let mut min_val = f32::MAX;
            let mut max_val = f32::MIN;

            // El compilador auto-vectoriza este bucle simple a instrucciones AVX2/NEON
            for &val in chunk {
                if val < min_val { min_val = val; }
                if val > max_val { max_val = val; }
            }

            MinMaxPair { min: min_val, max: max_val }
        })
        .collect()
}

```

### 6.3. Shader WGSL de Renderizado Instanciado (`crates/petaplot-render/src/shaders/line.wgsl`)

```wgsl
struct CameraUniform {
    transform_matrix: mat4x4<f32>,
    color: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct InstanceInput {
    @location(0) x_pos: f32,
    @location(1) y_min: f32,
    @location(2) y_max: f32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
};

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    instance: InstanceInput,
) -> VertexOutput {
    var out: VertexOutput;
    
    // Asignación de y según el índice de vértice (0 = min, 1 = max)
    var y_val: f32 = instance.y_min;
    if (vertex_index % 2u == 1u) {
        y_val = instance.y_max;
    }

    let raw_position = vec4<f32>(instance.x_pos, y_val, 0.0, 1.0);
    out.clip_position = camera.transform_matrix * raw_position;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return camera.color;
}

```

### 6.4. Plantilla del `README.md` Inicial del Repositorio

```markdown
# PetaPlot

[![CI Status](https://github.com/petaplot/petaplot/workflows/CI/badge.svg)](https://github.com/petaplot/petaplot/actions)
[![Crates.io](https://img.shields.io/crates/v/petaplot-cli.svg)](https://crates.io/crates/petaplot-cli)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](#license)

> **PetaPlot** is an open-source, ultra-fast time-series visualizer engineered for terabyte-scale datasets.

## Key Features

- **Zero-RAM Overhead:** Uses memory-mapped files (`mmap`) to open multi-gigabyte files instantly (<10ms).
- **GPU-Accelerated:** Renders at 144+ FPS powered by `wgpu` (Vulkan, Metal, Direct3D 12).
- **Sub-millisecond Latency:** Instant zoom and pan via vertex-shader matrix transformations.
- **Speculative Pre-fetching:** Predictive LRU caching for smooth, zero-stutter navigation.
- **Cross-Platform:** Native support for Linux, macOS, and Windows.

## Quick Start

```bash
# Install via Cargo
cargo install petaplot-cli

# Open a massive time-series file
petaplot telemetry_data.arrow

```

## Architecture Overview

PetaPlot is built as a modular Cargo Workspace:

* `petaplot-core`: Headless data processing engine, LOD pyramid generation, and SIMD kernels.
* `petaplot-render`: Low-level `wgpu` rendering pipelines and instance buffers.
* `petaplot-cli`: Cross-platform desktop application built with `egui`.

## License

Licensed under either of:

* Apache License, Version 2.0 ([LICENSE-APACHE](https://www.google.com/search?q=LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT License ([LICENSE-MIT](https://www.google.com/search?q=LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

```

---

## 7. Fases de Ejecución del Agente de Programación

1. **Fase 1 (Core & Storage):** Implementar `petaplot-core` con la abstracción `mmap`, pruebas unitarias de decimación SIMD en `rayon` y benchmarks de velocidad de lectura con `criterion`.
2. **Fase 2 (Pipeline de Renderizado):** Crear `petaplot-render` configurando `wgpu`, el shader WGSL para instanciado de líneas y el buffer uniforme de cámara.
3. **Fase 3 (UI & Integración CLI):** Desarrollar `petaplot-cli` integrando `egui` y coordinando los hilos de I/O y renderizado a través de canales de mensajes `crossbeam-channel`.
4. **Fase 4 (CI/CD):** Configurar GitHub Actions en `.github/workflows/` para compilación y empaquetado automático de binarios en Linux (`.deb`), macOS (`.dmg`) y Windows (`.exe`).

<Elicitation label="Generar la configuración de CI/CD para GitHub Actions" query="Escribe el archivo .github/workflows/ci.yml para compilar y testear PetaPlot automáticamente en Linux, macOS y Windows."/>
<Elicitation label="Profundizar en la integración de Apache Arrow y mmap" query="¿Cómo implementar el soporte para lectura zero-copy de archivos Apache ArrowIPC en petaplot-core?"/>

```