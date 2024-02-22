use bevy::{
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    prelude::*,
};
use bevy_egui::{egui, EguiContexts};

pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, fps_counter);
        std::fs::create_dir_all("graphs").expect("Failed to create graphs directory");
        print_render_graph(app);
        print_schedule_graphs::<Update>(app, Update);
        print_schedule_graphs::<PreUpdate>(app, PreUpdate);
        print_schedule_graphs::<PostUpdate>(app, PostUpdate);
        print_schedule_graphs::<Startup>(app, Startup);
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

pub fn print_render_graph(app: &mut App) {
    let dot = bevy_mod_debugdump::render_graph_dot(app, &Default::default());
    std::fs::write("graphs/RenderGraph.dot", dot).expect("Failed to write RenderGraph.dot");
    println!("Render graph written to graphs/RenderGraph.dot");
}

pub fn print_schedule_graphs<T: bevy::ecs::schedule::ScheduleLabel + Clone>(
    app: &mut App,
    label: T,
) {
    let dot = bevy_mod_debugdump::schedule_graph_dot(app, label.clone(), &Default::default());
    std::fs::write(format!("graphs/{:?}.dot", label), dot)
        .expect(format!("Failed to write {:?}.dot", label).as_str());
    println!("Schedule graph written to graphs/{:?}.dot", label);
}
