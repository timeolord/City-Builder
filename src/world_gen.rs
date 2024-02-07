use bevy::{
    prelude::*,
    tasks::{block_on, AsyncComputeTaskPool, Task},
};

mod erosion;
mod heightmap;
mod noise_generator;
use crate::{utils::math::AsF32, GameState};

use self::{
    erosion::{erode_heightmap, ErosionEvent},
    heightmap::Heightmap,
    noise_generator::{noise_function, NoiseFunction, NoiseSettings},
};
use bevy_egui::{egui, EguiContexts};

//Steps of world gen:
// 1. Generate height map # DONE
// 2. Generate mesh from height map
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
        app.add_systems(OnExit(GameState::WorldGeneration), exit);
    }
}

type WorldSize = [u32; 2];

const CHUNK_SIZE: u32 = 64;

#[derive(Resource, Clone, Copy, PartialEq, Eq)]
pub struct WorldGenSettings {
    world_size: WorldSize,
    noise_settings: NoiseSettings,
    erosion_amount: u32,
}

impl Default for WorldGenSettings {
    fn default() -> Self {
        let world_size = [8, 8];
        Self {
            world_size,
            noise_settings: NoiseSettings::new(world_size),
            erosion_amount: 50,
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
        /* self.erosion_progress;
        self.heightmap_progress */
    }
}

fn init(mut commands: Commands) {
    commands.init_resource::<WorldGenSettings>();
    commands.insert_resource(Heightmap::new(WorldGenSettings::default().world_size));
    commands.init_resource::<HeightmapLoadBar>();
}

fn exit() {}

fn generate_heightmap(
    mut heightmap: ResMut<Heightmap>,
    world_settings: Res<WorldGenSettings>,
    mut tasks: Local<Vec<Task<Vec<([u32; 2], f64)>>>>,
    mut previous_world_settings: Local<Option<WorldGenSettings>>,
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
                    let mut results = Vec::with_capacity((CHUNK_SIZE * CHUNK_SIZE) as usize);
                    for x in 0..CHUNK_SIZE {
                        for y in 0..CHUNK_SIZE {
                            let x = x + chunk_x * CHUNK_SIZE;
                            let y = y + chunk_y * CHUNK_SIZE;
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
    mut heightmap_image_handle: Local<Option<egui::load::SizedTexture>>,
    mut world_settings: ResMut<WorldGenSettings>,
    mut seed_string: Local<String>,
    heightmap_load_bar: Res<HeightmapLoadBar>,
) {
    if heightmap.is_changed() {
        *heightmap_image_handle = None;
    }

    if heightmap_image_handle.is_none() {
        let heightmap_image = heightmap.clone().as_bevy_image();
        let heightmap_bevy_handle = asset_server.add(heightmap_image);
        let heightmap_egui_handle = contexts.add_image(heightmap_bevy_handle);
        *heightmap_image_handle = Some(egui::load::SizedTexture::new(
            heightmap_egui_handle,
            heightmap.size().as_f32(),
        ));
    }

    if seed_string.is_empty() {
        *seed_string = world_settings.noise_settings.seed.to_string();
    }

    egui::SidePanel::left("World_Gen_Settings")
        .resizable(false)
        .show(contexts.ctx_mut(), |ui| {
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

                    ui.label("Noise Scale");
                    ui.add(
                        egui::Slider::new(
                            &mut world_settings.noise_settings.noise_scale,
                            0.0..=0.01,
                        )
                        .clamp_to_range(true),
                    );
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
                        egui::Slider::new(&mut world_settings.erosion_amount, 0..=100)
                            .clamp_to_range(true),
                    );
                    ui.end_row();
                });
        });
    egui::SidePanel::right("Heightmap_Image")
        .resizable(false)
        .show(contexts.ctx_mut(), |ui| {
            ui.with_layout(
                egui::Layout::top_down(egui::Align::Center),
                |ui| {
                    //println!("{}", heightmap_load_bar.progress());
                    ui.label("Heightmap");
                    if heightmap_load_bar.heightmap_progress >= 1.0 {
                        let heightmap_image = egui::Image::new(heightmap_image_handle.unwrap())
                            .fit_to_exact_size([512.0, 512.0].into());
                        ui.add(heightmap_image);
                    }
                    if heightmap_load_bar.progress() < 1.0 {
                        let mut load_bar = egui::ProgressBar::new(heightmap_load_bar.progress())
                                .desired_width(512.0);
                        if heightmap_load_bar.heightmap_progress < 1.0 {
                            load_bar = load_bar.text("Generating Heightmap");
                        }
                        else if heightmap_load_bar.erosion_progress < 1.0 {
                            load_bar = load_bar.text("Eroding Heightmap");
                        }
                        ui.add(load_bar);
                    }
                },
            );
        });
}
