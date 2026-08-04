use bytemuck::{Pod, Zeroable};

/// Uniform Buffer conteniendo la matriz de transformación del Viewport y el color de renderizado.
/// Tamaño total: 96 bytes (alineado a 16 bytes para std140 / WGSL).
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct CameraUniform {
    pub transform_matrix: [[f32; 4]; 4],
    pub color: [f32; 4],
    pub line_width: f32,
    pub _padding: [f32; 3],
}

impl Default for CameraUniform {
    fn default() -> Self {
        Self {
            transform_matrix: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            color: [0.2, 0.6, 1.0, 1.0], // Azul neón por defecto
            line_width: 1.5,
            _padding: [0.0; 3],
        }
    }
}

/// Cámara 2D para controlar el Zoom y Pan en el gráfico con $0\text{ ms}$ de latencia en CPU.
pub struct ViewportCamera {
    pub x_min: f64,
    pub x_max: f64,
    pub y_min: f32,
    pub y_max: f32,
    pub color: [f32; 4],
    pub line_width: f32,
}

impl Default for ViewportCamera {
    fn default() -> Self {
        Self {
            x_min: 0.0,
            x_max: 100.0,
            y_min: -1.0,
            y_max: 1.0,
            color: [0.2, 0.7, 1.0, 1.0],
            line_width: 1.5,
        }
    }
}

impl ViewportCamera {
    pub fn new(x_min: f64, x_max: f64, y_min: f32, y_max: f32) -> Self {
        Self {
            x_min,
            x_max,
            y_min,
            y_max,
            ..Default::default()
        }
    }

    /// Desplaza la cámara en el eje X (Pan horizontal).
    pub fn pan_x(&mut self, delta_x: f64) {
        self.x_min += delta_x;
        self.x_max += delta_x;
    }

    /// Aplica Zoom centrado en una coordenada $X$.
    pub fn zoom_x(&mut self, center_x: f64, factor: f64) {
        let span = self.x_max - self.x_min;
        let new_span = (span * factor).max(1e-6);

        let ratio = if span > 0.0 {
            (center_x - self.x_min) / span
        } else {
            0.5
        };

        self.x_min = center_x - ratio * new_span;
        self.x_max = self.x_min + new_span;
    }

    /// Genera la matriz de transformación ortográfica 4x4 mapeando las coordenadas del dataset a NDC (Normalized Device Coordinates: $[-1, 1]$).
    pub fn build_uniform(&self) -> CameraUniform {
        let span_x = (self.x_max - self.x_min) as f32;
        let span_y = self.y_max - self.y_min;

        let scale_x = if span_x > 0.0 { 2.0 / span_x } else { 1.0 };
        let scale_y = if span_y > 0.0 { 2.0 / span_y } else { 1.0 };

        let offset_x = -1.0 - (self.x_min as f32) * scale_x;
        let offset_y = -1.0 - self.y_min * scale_y;

        let transform_matrix = [
            [scale_x, 0.0, 0.0, 0.0],
            [0.0, scale_y, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [offset_x, offset_y, 0.0, 1.0],
        ];

        CameraUniform {
            transform_matrix,
            color: self.color,
            line_width: self.line_width,
            _padding: [0.0; 3],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_camera_uniform_transformation() {
        let camera = ViewportCamera::new(0.0, 100.0, -1.0, 1.0);
        let uniform = camera.build_uniform();

        // En x = 50.0, la posición transformada en pantalla debe ser NDC x = 0.0
        let scale_x = uniform.transform_matrix[0][0];
        let offset_x = uniform.transform_matrix[3][0];
        let ndc_x = 50.0 * scale_x + offset_x;

        assert!((ndc_x - 0.0).abs() < 1e-5);
    }
}
