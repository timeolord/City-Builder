use std::path::{Path, PathBuf};

use bevy::prelude::*;

use crate::{
    save::{save_path, LoadEvent},
    GameState,
};
use bevy_egui::{egui, EguiContexts};
use egui_file::FileDialog;

pub struct MainMenuPlugin;

impl Plugin for MainMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, main_menu.run_if(in_state(GameState::MainMenu)));
    }
}

fn main_menu(
    mut game_state: ResMut<NextState<GameState>>,
    mut contexts: EguiContexts,
    mut file_dialog: Local<Option<FileDialog>>,
    mut load_event: EventWriter<LoadEvent>,
) {
    let ctx = contexts.ctx_mut();
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
            let button = egui::Button::new("New Game").min_size([150.0, 65.0].into());
            if ui.add(button).clicked() {
                game_state.set(GameState::WorldGeneration);
            }
            let button = egui::Button::new("Load Game").min_size([150.0, 65.0].into());
            if ui.add(button).clicked() {
                if file_dialog.is_none() {
                    let mut dialog = FileDialog::open_file(Some(save_path()))
                        .show_new_folder(false)
                        .show_drives(false)
                        .show_rename(false)
                        .show_files_filter(Box::new(|str: &Path| {
                            str.extension().unwrap_or_default() == "save"
                        }));
                    dialog.open();
                    *file_dialog = Some(dialog);
                }
            }
            if file_dialog.is_some() {
                let dialog = file_dialog.as_mut().unwrap();
                dialog.show(ctx);
                let state = dialog.state();
                match state {
                    egui_file::State::Open => {}
                    egui_file::State::Closed | egui_file::State::Cancelled => {
                        *file_dialog = None;
                    }
                    egui_file::State::Selected => {
                        let event = LoadEvent(PathBuf::from(dialog.path().unwrap()));
                        load_event.send(event);
                        game_state.set(GameState::World);
                    }
                }
            }
        });
    });
}
