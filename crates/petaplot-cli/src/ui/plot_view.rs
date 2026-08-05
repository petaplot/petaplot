use egui::{CentralPanel, Context, Frame, PointerButton, Sense};
use crate::state::AppState;

pub struct PlotViewUi;

impl PlotViewUi {
    pub fn show(ctx: &Context, state: &mut AppState) {
        CentralPanel::default()
            .frame(Frame::dark_canvas(&ctx.style()))
            .show(ctx, |ui| {
                let (response, painter) = ui.allocate_painter(
                    ui.available_size(),
                    Sense::drag(),
                );

                let rect = response.rect;
                let width_px = rect.width();

                // Manejo de Interacción: Arrastrar ratón para Pan (desplazamiento horizontal)
                if response.dragged_by(PointerButton::Primary) {
                    let delta = response.drag_delta();
                    if width_px > 0.0 {
                        let span = state.camera.x_max - state.camera.x_min;
                        let delta_x_data = -(delta.x as f64 / width_px as f64) * span;
                        state.camera.pan_x(delta_x_data);
                    }
                }

                // Manejo de Interacción: Rueda de ratón para Zoom centrado en el cursor
                let scroll_delta = ctx.input(|i| i.raw_scroll_delta);
                if scroll_delta.y != 0.0 {
                    if let Some(hover_pos) = ctx.pointer_hover_pos() {
                        let relative_x = (hover_pos.x - rect.min.x) / width_px;
                        let span = state.camera.x_max - state.camera.x_min;
                        let mouse_data_x = state.camera.x_min + relative_x as f64 * span;

                        let zoom_factor = if scroll_delta.y > 0.0 { 0.85 } else { 1.15 };
                        state.camera.zoom_x(mouse_data_x, zoom_factor);
                    }
                }

                // Actualizar el predictor de velocidad e inercia
                let current_center = (state.camera.x_min + state.camera.x_max) / 2.0;
                state.prefetcher.update_position(current_center);

                // Selección dinámica del nivel LOD óptimo
                if let Some(ref pyramid) = state.pyramid {
                    let visible_samples = ((state.camera.x_max - state.camera.x_min).abs()) as usize;
                    state.active_lod = pyramid.select_optimal_lod(visible_samples, width_px, 1.0);
                }

                // Dibujo de rejilla indicadora overlay en el lienzo
                let grid_color = egui::Color32::from_rgba_unmultiplied(255, 255, 255, 15);
                let num_grid_lines = 10;
                for i in 1..num_grid_lines {
                    let x = rect.min.x + (rect.width() / num_grid_lines as f32) * i as f32;
                    painter.line_segment(
                        [egui::pos2(x, rect.min.y), egui::pos2(x, rect.max.y)],
                        egui::Stroke::new(1.0_f32, grid_color),
                    );
                }
            });
    }
}
