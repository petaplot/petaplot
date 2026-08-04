use std::sync::Arc;
use wgpu::{Adapter, Device, Instance, Queue};

/// Contexto principal de GPU administrando la instancia de wgpu, adaptador, dispositivo y cola de comandos.
pub struct RenderContext {
    pub instance: Instance,
    pub adapter: Adapter,
    pub device: Arc<Device>,
    pub queue: Arc<Queue>,
}

impl RenderContext {
    /// Inicializa el contexto de GPU de forma asíncrona seleccionando el backend nativo por hardware (Vulkan, Metal o D3D12).
    pub async fn new_async() -> Result<Self, String> {
        let instance = Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .ok_or_else(|| "No se encontró un adaptador GPU compatible en el sistema.".to_string())?;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("TeraPlot GPU Device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: wgpu::MemoryHints::Performance,
                },
                None,
            )
            .await
            .map_err(|e| format!("Error al crear el dispositivo GPU: {}", e))?;

        Ok(Self {
            instance,
            adapter,
            device: Arc::new(device),
            queue: Arc::new(queue),
        })
    }

    /// Inicialización bloqueante (sincrónica) para integraciones directas en hilos principales.
    pub fn new_blocking() -> Result<Self, String> {
        pollster::block_on(Self::new_async())
    }
}
