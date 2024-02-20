use std::path::Path;

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use image::{DynamicImage, RgbImage, RgbaImage};
use itertools::Itertools;
use strum::IntoEnumIterator;

use crate::GameState;

use super::{TerrainTextureAtlas, TerrainTextures, TerrainType};

pub struct AssetLoaderPlugin;

impl Plugin for AssetLoaderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TerrainTextures>();
        app.init_resource::<AssetLoadBar>();
        app.init_resource::<TerrainTextureAtlas>();
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
    mut terrain_textures: ResMut<TerrainTextures>,
) {
    for terrain_type in TerrainType::iter() {
        let mut file_path = Path::new("textures").join(terrain_type.to_string().to_lowercase());
        file_path.set_extension("png");
        let texture_handle = asset_server.load(file_path);
        terrain_textures[terrain_type] = texture_handle;
    }
}

fn check_assets(
    mut game_state: ResMut<NextState<GameState>>,
    terrain_textures: Res<TerrainTextures>,
    mut image_assets: ResMut<Assets<Image>>,
    mut asset_load_bar: ResMut<AssetLoadBar>,
    mut terrain_texture_atlas: ResMut<TerrainTextureAtlas>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut progress = 0.0;
    for image in terrain_textures.values() {
        match image_assets.get(image) {
            Some(_) => progress += 1.0 / TerrainType::iter().len() as f32,
            None => {}
        }
    }
    asset_load_bar.progress = progress;
    if progress >= 1.0 {
        //Create Texture Atlas
        let mut texture_atlas: Vec<u8> = Vec::new();
        let mut image_size = UVec2::new(0, 0);
        for image in terrain_textures.values() {
            let image = image_assets.get(image).unwrap();
            image_size = image.size();
            texture_atlas.append(&mut image.data.iter().cloned().collect_vec());
        }
        let image = RgbaImage::from_raw(
            image_size.x,
            TerrainType::iter().len() as u32 * image_size.y,
            texture_atlas,
        )
        .unwrap();
        let image = DynamicImage::ImageRgba8(image);
        let image = Image::from_dynamic(image, false);
        terrain_texture_atlas.handle = materials.add(StandardMaterial {
            base_color_texture: Some(image_assets.add(image)),
            alpha_mode: AlphaMode::Opaque,
            specular_transmission: 0.0,
            reflectance: 0.0,
            ..Default::default()
        });

        game_state.set(GameState::MainMenu);
    }
}
