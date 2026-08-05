use egui::{Color32, Context, SidePanel};
use crate::state::AppState;

pub struct ControlsUi;

impl ControlsUi {
    pub fn show(ctx: &Context, state: &mut AppState) {
        SidePanel::left("controls_panel")
            .resizable(true)
            .default_width(260.0)
            .show(ctx, |ui| {
                ui.heading("PetaPlot Controls");
                ui.separator();

                ui.collapsing("Métricas de Rendimiento", |ui| {
                    ui.horizontal(|ui| {
                        ui.label("FPS:");
                        let fps_color = if state.fps >= 55.0 {
                            Color32::GREEN
                        } else if state.fps >= 30.0 {
                            Color32::YELLOW
                        } else {
                            Color32::RED
                        };
                        ui.colored_label(fps_color, format!("{:.1} FPS", state.fps));
                    });

                    let frame_time_ms = if state.fps > 0.0 {
                        1000.0 / state.fps
                    } else {
                        0.0
                    };
                    ui.label(format!("Tiempo de Frame: {:.2} ms", frame_time_ms));
                    ui.label(format!("Velocidad Inercia: {:.1} px/s", state.prefetcher.current_velocity()));
                });

                ui.separator();

                ui.collapsing("Dataset y Pirámide LOD", |ui| {
                    ui.label(&state.status_message);

                    if let Some(ref pyramid) = state.pyramid {
                        ui.label(format!("Muestras Totales: {}", pyramid.total_samples));
                        ui.label(format!("Niveles LOD: {}", pyramid.num_levels()));
                        ui.label(format!("LOD Activo: Nivel {}", state.active_lod));
                    }

                    ui.add_space(8.0);
                    if ui.button("⚡ Cargar Demo (500,000 pts)").clicked() {
                        state.load_demo_dataset();
                    }
                });

                ui.separator();

                ui.collapsing("Estilo Visual", |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Grosor de Línea:");
                        ui.add(egui::Slider::new(&mut state.camera.line_width, 0.5..=5.0));
                    });

                    ui.label("Color de Renderizado:");
                    let mut color_array = [
                        state.camera.color[0],
                        state.camera.color[1],
                        state.camera.color[2],
                    ];
                    if ui.color_edit_button_rgb(&mut color_array).changed() {
                        state.camera.color[0] = color_array[0];
                        state.camera.color[1] = color_array[1];
                        state.camera.color[2] = color_array[2];
                    }
                });

                ui.separator();

                if ui.button("🔄 Resetear Vista (Zoom 1:1)").clicked() {
                    if let Some(ref pyramid) = state.pyramid {
                        state.camera.x_min = 0.0;
                        state.camera.x_max = pyramid.total_samples as f64;
                    }
                }
            });
    }
}
