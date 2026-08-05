use eframe::{App, Frame};
use egui::Context;
use crate::state::AppState;
use crate::ui::controls::ControlsUi;
use crate::ui::plot_view::PlotViewUi;

pub struct PetaApp {
    pub state: AppState,
}

impl Default for PetaApp {
    fn default() -> Self {
        let mut state = AppState::new();
        state.load_demo_dataset();
        Self { state }
    }
}

impl PetaApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self::default()
    }
}

impl App for PetaApp {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        self.state.tick_fps();

        ControlsUi::show(ctx, &mut self.state);
        PlotViewUi::show(ctx, &mut self.state);

        // Solicitar repintado continuo para garantizar 60-144 FPS estables durante interacciones
        ctx.request_repaint();
    }
}
