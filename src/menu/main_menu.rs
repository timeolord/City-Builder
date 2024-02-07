use bevy::prelude::*;

use crate::GameState;
use bevy_egui::{egui, EguiContexts};

pub struct MainMenuPlugin;

impl Plugin for MainMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, main_menu.run_if(in_state(GameState::MainMenu)));
    }
}

fn main_menu(mut game_state: ResMut<NextState<GameState>>, mut contexts: EguiContexts) {
    egui::CentralPanel::default().show(contexts.ctx_mut(), |ui| {
        ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
            let button = egui::Button::new("New Game").min_size([150.0, 65.0].into());
            if ui.add(button).clicked() {
                game_state.set(GameState::WorldGeneration);
            }
        });
    });
}
