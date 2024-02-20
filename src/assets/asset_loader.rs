use std::path::Path;

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use strum::IntoEnumIterator;

use crate::GameState;

use super::{TerrainTextures, TerrainType};

pub struct AssetLoaderPlugin;

impl Plugin for AssetLoaderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TerrainTextures>();
        app.init_resource::<AssetLoadBar>();
        app.add_systems(
            Update,
            (check_assets, display_ui).run_if(in_state(GameState::AssetLoading)),
        );
        app.add_systems(OnEnter(GameState::AssetLoading), start_load_assets);
        app.add_systems(OnExit(GameState::AssetLoading), exit);
    }
}

#[derive(Resource, Default)]
pub struct AssetLoadBar {
    pub progress: f32,
}

fn exit(mut commands: Commands) {
    commands.remove_resource::<AssetLoadBar>();
}

fn display_ui(mut contexts: EguiContexts, asset_load_bar: Res<AssetLoadBar>) {
    let ctx = contexts.ctx_mut();
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
            let mut load_bar = egui::ProgressBar::new(asset_load_bar.progress)
                .desired_width(ui.available_width() * 0.66);
            load_bar = load_bar.text("Loading Textures...");
            ui.add(load_bar);
        });
    });
}

fn start_load_assets(
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut terrain_textures: ResMut<TerrainTextures>,
) {
    for terrain_type in TerrainType::iter() {
        let mut file_path = Path::new("textures").join(terrain_type.to_string().to_lowercase());
        file_path.set_extension("png");
        let texture_handle = asset_server.load(file_path);
        let material = StandardMaterial {
            base_color_texture: Some(texture_handle.clone()),
            ..Default::default()
        };
        let material_handle = materials.add(material);
        terrain_textures[terrain_type] = (texture_handle, material_handle);
    }
}

fn check_assets(
    mut game_state: ResMut<NextState<GameState>>,
    terrain_textures: Res<TerrainTextures>,
    image_assets: Res<Assets<Image>>,
    mut asset_load_bar: ResMut<AssetLoadBar>,
) {
    let mut progress = 0.0;
    for (image, _) in terrain_textures.values() {
        match image_assets.get(image) {
            Some(_) => progress += 1.0 / TerrainType::iter().len() as f32,
            None => {}
        }
    }
    asset_load_bar.progress = progress;
    if progress >= 1.0 {
        game_state.set(GameState::MainMenu);
    }
}
