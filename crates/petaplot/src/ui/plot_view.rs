use egui::{CentralPanel, Color32, Context, Frame, PointerButton, Pos2, Sense, Shape, Stroke};
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
                let height_px = rect.height();

                if width_px <= 0.0 || height_px <= 0.0 {
                    return;
                }

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

                // Dibujo de rejilla indicadora overlay en el lienzo
                let grid_color = Color32::from_rgba_unmultiplied(255, 255, 255, 15);
                let num_v_lines = 10;
                for i in 1..num_v_lines {
                    let x = rect.min.x + (rect.width() / num_v_lines as f32) * i as f32;
                    painter.line_segment(
                        [Pos2::new(x, rect.min.y), Pos2::new(x, rect.max.y)],
                        Stroke::new(1.0_f32, grid_color),
                    );
                }
                let num_h_lines = 6;
                for i in 1..num_h_lines {
                    let y = rect.min.y + (rect.height() / num_h_lines as f32) * i as f32;
                    painter.line_segment(
                        [Pos2::new(rect.min.x, y), Pos2::new(rect.max.x, y)],
                        Stroke::new(1.0_f32, grid_color),
                    );
                }

                // Eje central Y = 0 (línea guía)
                let y_span = state.camera.y_max - state.camera.y_min;
                if y_span > 0.0 {
                    let zero_y_ratio = (0.0 - state.camera.y_min) / y_span;
                    if (0.0..=1.0).contains(&zero_y_ratio) {
                        let zero_py = rect.max.y - zero_y_ratio * rect.height();
                        painter.line_segment(
                            [Pos2::new(rect.min.x, zero_py), Pos2::new(rect.max.x, zero_py)],
                            Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 35)),
                        );
                    }
                }

                // Selección dinámica del nivel LOD óptimo y Renderizado de la Serie Temporal
                if let Some(ref pyramid) = state.pyramid {
                    let visible_samples = ((state.camera.x_max - state.camera.x_min).abs()) as usize;
                    state.active_lod = pyramid.select_optimal_lod(visible_samples, width_px, 1.0);

                    if let Ok(level) = pyramid.get_level(state.active_lod) {
                        let step_factor = level.step_factor;
                        let x_min = state.camera.x_min;
                        let x_max = state.camera.x_max;
                        let x_span = (x_max - x_min) as f32;

                        if x_span > 0.0 && y_span > 0.0 && !level.pairs.is_empty() {
                            let line_color = Color32::from_rgba_unmultiplied(
                                (state.camera.color[0] * 255.0) as u8,
                                (state.camera.color[1] * 255.0) as u8,
                                (state.camera.color[2] * 255.0) as u8,
                                (state.camera.color[3] * 255.0) as u8,
                            );
                            let stroke = Stroke::new(state.camera.line_width, line_color);

                            let start_idx = (x_min / step_factor as f64).floor().max(0.0) as usize;
                            let end_idx = ((x_max / step_factor as f64).ceil() as usize + 1).min(level.pairs.len());

                            let mut shapes: Vec<Shape> = Vec::with_capacity((end_idx.saturating_sub(start_idx)) * 2);
                            let mut prev_point: Option<Pos2> = None;

                            for i in start_idx..end_idx {
                                let pair = &level.pairs[i];
                                let x_data = i * step_factor;
                                let px = rect.min.x + ((x_data as f64 - x_min) as f32 / x_span) * rect.width();

                                let py_min = rect.max.y - ((pair.min - state.camera.y_min) / y_span) * rect.height();
                                let py_max = rect.max.y - ((pair.max - state.camera.y_min) / y_span) * rect.height();
                                let py_mid = (py_min + py_max) * 0.5;

                                let pt_mid = Pos2::new(px, py_mid);

                                // 1. Barra vertical Min-Max (Envelope del bin)
                                if (py_min - py_max).abs() > 0.5 {
                                    shapes.push(Shape::line_segment(
                                        [Pos2::new(px, py_min), Pos2::new(px, py_max)],
                                        stroke,
                                    ));
                                }

                                // 2. Línea de conexión entre bins adyacentes
                                if let Some(prev_pt) = prev_point {
                                    shapes.push(Shape::line_segment([prev_pt, pt_mid], stroke));
                                }

                                prev_point = Some(pt_mid);
                            }

                            painter.extend(shapes);
                        }
                    }
                }
            });
    }
}

