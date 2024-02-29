use std::{
    mem,
    path::{Path, PathBuf},
};

use bevy::{
    prelude::*,
    render::{
        render_resource::{
            BufferDescriptor, BufferUsages,
        },
        renderer::RenderDevice,
    },
    tasks::{block_on, AsyncComputeTaskPool, Task},
};
use egui_file::FileDialog;
use serde::{Deserialize, Serialize};

pub mod erosion;
pub mod heightmap;
pub mod mesh_gen;
pub mod noise_gen;
pub mod terrain_material;

use crate::{
    save::{save_path, SaveEvent},
    utils::math::AsF32,
    GameState,
};

use self::{
    erosion::{erode_heightmap, ErosionEvent},
    heightmap::{Heightmap, HeightmapImage},
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
// 3. Generate ground textures from height map # DONE
// 4. Spawn trees

pub struct WorldGenPlugin;

impl Plugin for WorldGenPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<ErosionEvent>();
        app.add_systems(OnEnter(GameState::WorldGeneration), init);
        app.add_systems(
            Update,
            (
                generate_heightmap,
                erode_heightmap,
                display_ui.run_if(resource_exists::<HeightmapImage>),
            )
                .run_if(in_state(GameState::WorldGeneration)),
        );
        app.add_systems(
            PostUpdate,
            update_heightmap_image.run_if(resource_exists::<Heightmap>),
        );
        app.add_systems(
            Update,
            (generate_world_mesh).run_if(in_state(GameState::World)),
        );
        app.add_systems(OnExit(GameState::WorldGeneration), exit);
    }
}

type WorldSize = [u32; 2];

pub const CHUNK_SIZE: u32 = 128;
pub const HEIGHTMAP_CHUNK_SIZE: u32 = CHUNK_SIZE + 1;

fn update_heightmap_image(
    mut commands: Commands,
    heightmap: ResMut<Heightmap>,
    world_settings: Res<WorldSettings>,
    heightmap_image: Option<ResMut<HeightmapImage>>,
    render_device: Res<RenderDevice>,
    mut image_assets: ResMut<Assets<Image>>,
    mut counter: Local<u8>,
) {
    *counter = counter.saturating_add(1);
    if heightmap_image.is_none() {
        /* let pixel = [0.25f32, 0.5, 0.75, 1.0];
        let pixel_bytes = pixel.map(|x| x.to_ne_bytes()); */
        /* let mut vertices_image = Image::new_fill(
            Extent3d {
                width: heightmap.size()[0],
                height: heightmap.size()[1],
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            pixel_bytes.flatten(),
            TextureFormat::Rgba32Float,
            RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
        );
        vertices_image.texture_descriptor.usage = TextureUsages::COPY_DST | TextureUsages::COPY_SRC
            | TextureUsages::STORAGE_BINDING
            | TextureUsages::TEXTURE_BINDING; */
        let vertices_length =
            (heightmap.size()[0] * heightmap.size()[1] * 4 * 3 * mem::size_of::<f32>() as u32)
                as u64;
        println!("Vertices Length: {}", vertices_length);
        /* println!("Vertices Length: {}, {}", vertices_length, vertices_image.data.len() as u64); */
        let buffer = render_device.create_buffer(&BufferDescriptor {
            label: Some("Heightmap Vertices Buffer"),
            size: vertices_length,
            usage: BufferUsages::COPY_SRC | BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        let staging_buffer = render_device.create_buffer(&BufferDescriptor {
            label: Some("Heightmap Vertices Staging Buffer"),
            size: vertices_length,
            usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        commands.insert_resource(HeightmapImage {
            image: image_assets.add(heightmap.clone().as_bevy_image()),
            /* vertices: image_assets.add(vertices_image), */
            size: heightmap.size().into(),
            world_size: world_settings.tile_world_size().into(),
            buffer,
            staging_buffer,
        });
    } else if heightmap.is_changed() && *counter > 10 {
        let old_image = image_assets
            .get_mut(heightmap_image.as_ref().unwrap().image.clone_weak())
            .unwrap();
        let new_image = heightmap.clone().as_bevy_image();
        *old_image = new_image;
        *counter = 0;
    }
}

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

impl WorldSettings {
    fn seed(&self) -> u32 {
        self.noise_settings.seed
    }
    fn tile_world_size(&self) -> WorldSize {
        let mut world_size = self.world_size;
        world_size[0] *= CHUNK_SIZE;
        world_size[1] *= CHUNK_SIZE;
        world_size
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
                heightmap[index] = noise as f32;
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
    heightmap: Res<HeightmapImage>,
    mut contexts: EguiContexts,
    mut egui_heightmap_image_handle: Local<Option<TextureId>>,
    mut world_settings: ResMut<WorldSettings>,
    mut seed_string: Local<String>,
    heightmap_load_bar: Res<HeightmapLoadBar>,
    mut game_state: ResMut<NextState<GameState>>,
    mut save_event: EventWriter<SaveEvent>,
    mut file_dialog: Local<Option<FileDialog>>,
    mut frame_counter: Local<u8>,
) {
    *frame_counter = frame_counter.saturating_add(1);
    if egui_heightmap_image_handle.is_none() {
        let heightmap_egui_handle = contexts.add_image(heightmap.image.clone_weak());
        *egui_heightmap_image_handle = Some(heightmap_egui_handle);
    }

    /* //Update the image if the heightmap has changed every 30 frames
    if heightmap.is_changed() && (*frame_counter > 30 || heightmap_load_bar.progress() >= 1.0) {
        let heightmap_image = heightmap.clone().as_bevy_image();
        let heightmap_bevy_handle = asset_server
            .get_mut(bevy_heightmap_image_handle.as_ref().unwrap().clone())
            .unwrap();
        *heightmap_bevy_handle = heightmap_image;
        *frame_counter = 0;
    } */

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
                            {
                                dialog = dialog.show_drives(false);
                            }
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
                        <[f32; 2] as Into<egui::Vec2>>::into(heightmap.size.to_array().as_f32()),
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
    #[cfg(unix)]
    {
        coz::progress!("Display UI");
    }
}
