use bevy::{
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    prelude::*,
};
use bevy_egui::{egui, EguiContexts};

pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, fps_counter);
    }
}

fn fps_counter(mut contexts: EguiContexts, diagnostics: Res<DiagnosticsStore>) {
    let ctx = contexts.ctx_mut();
    egui::Window::new("FPS Counter")
        .resizable(false)
        .title_bar(false)
        .show(ctx, |ui| {
            if let Some(fps) = diagnostics
                .get(FrameTimeDiagnosticsPlugin::FPS)
                .and_then(|fps| fps.smoothed())
            {
                ui.label(format!("FPS: {:.2}", fps));
            }
        });
}
