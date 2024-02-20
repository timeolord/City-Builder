use std::path::{Path, PathBuf};

use bevy::{
    prelude::*,
    tasks::{block_on, AsyncComputeTaskPool, Task},
};
use egui_file::FileDialog;
use serde::{Deserialize, Serialize};

pub mod erosion;
pub mod heightmap;
pub mod mesh_gen;
pub mod noise_gen;

use crate::{
    save::{save_path, SaveEvent},
    utils::math::AsF32,
    GameState,
};

use self::{
    erosion::{erode_heightmap, ErosionEvent},
    heightmap::Heightmap,
    mesh_gen::generate_world_mesh,
    noise_gen::{noise_function, NoiseFunction, NoiseSettings},
};
use bevy_egui::{
    egui::{self, TextureId},
    EguiContexts,
};

//Steps of world gen:
// 1. Generate height map # DONE
// 2. Generate mesh from height map # DONE
// 2a. Generate water mesh from height map
// 3. Generate ground textures from height map
// 4. Spawn trees

pub struct WorldGenPlugin;

impl Plugin for WorldGenPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<ErosionEvent>();
        app.add_systems(OnEnter(GameState::WorldGeneration), init);
        app.add_systems(
            Update,
            (generate_heightmap, erode_heightmap, display_ui)
                .run_if(in_state(GameState::WorldGeneration)),
        );
        app.add_systems(
            Update,
            generate_world_mesh.run_if(in_state(GameState::World)),
        );
        app.add_systems(OnExit(GameState::WorldGeneration), exit);
    }
}

type WorldSize = [u32; 2];

pub const CHUNK_SIZE: u32 = 128;
pub const HEIGHTMAP_CHUNK_SIZE: u32 = CHUNK_SIZE + 1;

#[derive(Resource, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldSettings {
    pub world_size: WorldSize,
    pub noise_settings: NoiseSettings,
    pub erosion_amount: u32,
}

impl Default for WorldSettings {
    fn default() -> Self {
        let world_size = [16, 16];
        Self {
            world_size,
            noise_settings: NoiseSettings::new(world_size),
            erosion_amount: 500,
        }
    }
}

#[derive(Resource, Default)]
pub struct HeightmapLoadBar {
    heightmap_progress: f32,
    erosion_progress: f32,
}
impl HeightmapLoadBar {
    pub fn progress(&self) -> f32 {
        (self.heightmap_progress + self.erosion_progress) / 2.0
    }
}

fn init(mut commands: Commands) {
    commands.init_resource::<WorldSettings>();
    commands.insert_resource(Heightmap::new(WorldSettings::default().world_size));
    commands.init_resource::<HeightmapLoadBar>();
}

fn exit(mut commands: Commands) {
    commands.remove_resource::<HeightmapLoadBar>();
}

fn generate_heightmap(
    mut heightmap: ResMut<Heightmap>,
    world_settings: Res<WorldSettings>,
    mut tasks: Local<Vec<Task<Vec<([u32; 2], f64)>>>>,
    mut previous_world_settings: Local<Option<WorldSettings>>,
    mut heightmap_load_bar: ResMut<HeightmapLoadBar>,
    mut erosion_event: EventWriter<ErosionEvent>,
    mut working: Local<bool>,
) {
    let world_size = world_settings.world_size;

    if *working {
        if tasks.is_empty() {
            heightmap_load_bar.heightmap_progress = 0.0;
        } else {
            //Update the load bar
            heightmap_load_bar.heightmap_progress =
                tasks.iter().filter(|task| task.is_finished()).count() as f32 / tasks.len() as f32;
        }
    }

    //Checks tasks first to give one frame of processing time to the tasks
    if heightmap_load_bar.heightmap_progress >= 1.0 && *working {
        //Tasks are finished, process the results
        for task in &mut tasks {
            let result = block_on(task);
            for (index, noise) in result {
                heightmap[index] = noise;
            }
        }
        tasks.clear();
        *working = false;
        //Trigger the erosion event
        erosion_event.send(ErosionEvent);
    }

    if previous_world_settings.is_none() || *world_settings != previous_world_settings.unwrap() {
        *working = true;
        tasks.clear();
        let thread_pool = AsyncComputeTaskPool::get();
        let noise_settings = world_settings.noise_settings;

        //Seperate each chunk into its own task to be processed in parallel, and over multiple frames
        for chunk_y in 0..world_size[0] {
            for chunk_x in 0..world_size[1] {
                let task = thread_pool.spawn(async move {
                    let perlin = noise_function(noise_settings);
                    let mut results =
                        Vec::with_capacity((HEIGHTMAP_CHUNK_SIZE * HEIGHTMAP_CHUNK_SIZE) as usize);
                    for x in 0..HEIGHTMAP_CHUNK_SIZE {
                        for y in 0..HEIGHTMAP_CHUNK_SIZE {
                            let x = x + chunk_x * HEIGHTMAP_CHUNK_SIZE;
                            let y = y + chunk_y * HEIGHTMAP_CHUNK_SIZE;
                            let result = ([x, y], perlin.get([x, y]));
                            results.push(result);
                        }
                    }
                    results
                });
                tasks.push(task);
            }
        }
    }

    *previous_world_settings = Some(world_settings.clone());
}

fn display_ui(
    mut asset_server: ResMut<Assets<Image>>,
    heightmap: Res<Heightmap>,
    mut contexts: EguiContexts,
    mut bevy_heightmap_image_handle: Local<Option<Handle<Image>>>,
    mut egui_heightmap_image_handle: Local<Option<TextureId>>,
    mut world_settings: ResMut<WorldSettings>,
    mut seed_string: Local<String>,
    heightmap_load_bar: Res<HeightmapLoadBar>,
    mut game_state: ResMut<NextState<GameState>>,
    mut save_event: EventWriter<SaveEvent>,
    mut file_dialog: Local<Option<FileDialog>>,
    mut frame_counter: Local<u8>,
) {
    *frame_counter += 1;
    if bevy_heightmap_image_handle.is_none() || egui_heightmap_image_handle.is_none() {
        let heightmap_image = heightmap.clone().as_bevy_image();
        let heightmap_bevy_handle = asset_server.add(heightmap_image);
        *bevy_heightmap_image_handle = Some(heightmap_bevy_handle.clone());
        let heightmap_egui_handle = contexts.add_image(heightmap_bevy_handle);
        *egui_heightmap_image_handle = Some(heightmap_egui_handle);
    }

    //Update the image if the heightmap has changed every 10 frames
    if heightmap.is_changed() && (*frame_counter > 30 || heightmap_load_bar.progress() >= 1.0) {
        let heightmap_image = heightmap.clone().as_bevy_image();
        let heightmap_bevy_handle = asset_server
            .get_mut(bevy_heightmap_image_handle.as_ref().unwrap().clone())
            .unwrap();
        *heightmap_bevy_handle = heightmap_image;
        *frame_counter = 0;
    }

    if seed_string.is_empty() {
        *seed_string = world_settings.noise_settings.seed.to_string();
    }
    let ctx = contexts.ctx_mut();

    egui::SidePanel::left("World_Gen_Settings")
        .resizable(false)
        .show(ctx, |ui| {
            egui::Grid::new("World_Setting_Menu")
                .num_columns(2)
                .show(ui, |ui| {
                    ui.label("Seed");
                    let seed_text_box = ui.add(
                        egui::TextEdit::singleline(&mut *seed_string)
                            .desired_width(100.0)
                            .char_limit(u32::MAX.to_string().len()),
                    );
                    if seed_text_box.lost_focus()
                        || ui.input(|key| key.key_pressed(egui::Key::Enter))
                    {
                        match seed_string.parse() {
                            Ok(seed) => world_settings.noise_settings.seed = seed,
                            Err(_) => {
                                *seed_string = world_settings.noise_settings.seed.to_string();
                            }
                        }
                    }
                    ui.end_row();

                    ui.label("Hilliness");
                    ui.add(
                        egui::Slider::new(&mut world_settings.noise_settings.hilliness, 0.0..=1.0)
                            .clamp_to_range(true),
                    );
                    ui.end_row();

                    ui.label("Mountain Amount");
                    ui.add(
                        egui::Slider::new(
                            &mut world_settings.noise_settings.mountain_amount,
                            0..=10,
                        )
                        .clamp_to_range(true),
                    );
                    ui.end_row();

                    ui.label("Mountain Size");
                    ui.add(
                        egui::Slider::new(
                            &mut world_settings.noise_settings.mountain_size,
                            50.0..=200.0,
                        )
                        .clamp_to_range(true),
                    );
                    ui.end_row();

                    ui.label("Erosion");
                    ui.add(
                        egui::Slider::new(&mut world_settings.erosion_amount, 0..=1000)
                            .clamp_to_range(true),
                    );
                    ui.end_row();
                });
            if heightmap_load_bar.progress() >= 1.0 {
                ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                    let button = egui::Button::new("Save Heightmap").min_size([150.0, 65.0].into());
                    if ui.add(button).clicked() {
                        if file_dialog.is_none() {
                            let mut dialog = FileDialog::save_file(Some(save_path()))
                                .show_new_folder(false)
                                .show_rename(false)
                                .show_files_filter(Box::new(|str: &Path| {
                                    str.extension().unwrap_or_default() == "save"
                                }));
                            #[cfg(windows)]
                            let mut dialog = dialog.show_drives(false);

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
                                let mut path = PathBuf::from(dialog.path().unwrap());
                                path.set_extension("save");
                                let event = SaveEvent(path);
                                save_event.send(event);
                            }
                        }
                    }
                });
            }
        });
    egui::SidePanel::right("Heightmap_Image")
        .resizable(false)
        .show(contexts.ctx_mut(), |ui| {
            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                if heightmap_load_bar.heightmap_progress >= 1.0 {
                    let heightmap_image = egui::Image::new(egui::load::SizedTexture::new(
                        egui_heightmap_image_handle.unwrap(),
                        heightmap.size().as_f32(),
                    ))
                    .fit_to_exact_size([512.0, 512.0].into());
                    ui.add(heightmap_image);
                }
                if heightmap_load_bar.progress() < 1.0 {
                    let mut load_bar =
                        egui::ProgressBar::new(heightmap_load_bar.progress()).desired_width(512.0);
                    if heightmap_load_bar.heightmap_progress < 1.0 {
                        load_bar = load_bar.text("Generating Heightmap");
                    } else if heightmap_load_bar.erosion_progress < 1.0 {
                        load_bar = load_bar.text("Eroding Heightmap");
                    }
                    ui.add(load_bar);
                } else {
                    ui.centered_and_justified(|ui| {
                        let button = egui::Button::new("New Game").min_size([150.0, 65.0].into());
                        if ui.add(button).clicked() {
                            game_state.set(GameState::World);
                        }
                    });
                }
            });
        });
}
