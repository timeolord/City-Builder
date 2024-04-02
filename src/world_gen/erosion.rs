use bevy::{
    prelude::*,
    render::render_resource::{ShaderRef, ShaderType},
};

use bevy_app_compute::prelude::{
    AppComputeWorker, AppComputeWorkerBuilder, ComputeShader, ComputeWorker,
};
use bytemuck::NoUninit;

use itertools::Itertools;

use rand::{rngs::StdRng, SeedableRng};
use rand_distr::{Distribution, Uniform};
use std::fmt::Debug;

use super::{
    consts::CHUNK_WORLD_SIZE, heightmap::Heightmap, HeightmapLoadBar, WorldSettings,
    HEIGHTMAP_CHUNK_SIZE,
};

use std::time::Instant;

pub const MAX_DROPLET_SIZE: u32 = 12;
pub const MIN_DROPLET_SIZE: u32 = 2;
pub const EROSION_WORKGROUP_SIZE: u64 = 64;
pub const EROSION_DISPATCH_SIZE: u64 = 16;
pub const MAX_EROSION_STEPS: u64 = 500;

#[derive(Event)]
pub struct ErosionEvent;

#[derive(Debug, Clone, Copy, ShaderType, Default, NoUninit)]
#[repr(C)]
pub struct Droplet {
    position_x: u32,
    position_y: u32,
    radius: u32,
    sediment: f32,
    water: f32,
    speed: f32,
    direction_x: f32,
    direction_y: f32,
}

#[derive(TypePath)]
struct ErosionShader;

impl ComputeShader for ErosionShader {
    fn shader() -> ShaderRef {
        "shaders/terrain_erosion.wgsl".into()
    }
    fn dependencies() -> Vec<ShaderRef> {
        vec!["shaders/constants.wgsl".into()]
    }
}

#[derive(Resource)]
pub struct ErosionComputeWorker;

#[derive(Debug, Clone, Copy)]
pub enum ErosionComputeFields {
    Droplets,
    Results,
}

impl ComputeWorker for ErosionComputeWorker {
    type Fields = ErosionComputeFields;
    fn build(app: &mut App) -> AppComputeWorker<Self> {
        AppComputeWorkerBuilder::new(app)
            .add_rw_storage(
                Self::Fields::Droplets,
                &vec![
                    Droplet::default();
                    EROSION_DISPATCH_SIZE as usize * EROSION_WORKGROUP_SIZE as usize
                ],
            )
            .add_staging(
                Self::Fields::Results,
                &vec![
                    0.0f32;
                    (CHUNK_WORLD_SIZE[0]
                        * HEIGHTMAP_CHUNK_SIZE as u32
                        * CHUNK_WORLD_SIZE[1]
                        * HEIGHTMAP_CHUNK_SIZE as u32) as usize
                ],
            )
            .add_pass::<ErosionShader>(
                [EROSION_DISPATCH_SIZE as u32, 1, 1],
                &[Self::Fields::Droplets, Self::Fields::Results],
            )
            .one_shot()
            .set_wait_mode(false)
            .build()
    }
}

pub fn gpu_erode_heightmap(
    mut erosion_worker: ResMut<AppComputeWorker<ErosionComputeWorker>>,
    settings: Res<WorldSettings>,
    mut heightmap: ResMut<Heightmap>,
    mut heightmap_load_bar: ResMut<HeightmapLoadBar>,
    mut erosion_counter: Local<u32>,
    mut erosion_event: EventReader<ErosionEvent>,
    mut working: Local<bool>,
    mut benchmark: Local<Option<Instant>>,
    mut rng: Local<Option<StdRng>>,
) {
    let erosion_chunks = settings.erosion_amount;
    let erosion_chunk_size = EROSION_DISPATCH_SIZE * EROSION_WORKGROUP_SIZE;

    if erosion_event.read().count() > 0 {
        *erosion_counter = erosion_chunks;
        heightmap_load_bar.erosion_progress = 0.0;
        *working = true;
        *benchmark = Some(Instant::now());
        *rng = Some(StdRng::seed_from_u64(settings.noise_settings.seed as u64));

        let map_size = [
            (CHUNK_WORLD_SIZE[0] * HEIGHTMAP_CHUNK_SIZE),
            (CHUNK_WORLD_SIZE[1] * HEIGHTMAP_CHUNK_SIZE),
        ];

        let position_sampler = Uniform::new(0, map_size[0]);
        let radius_sampler = Uniform::new_inclusive(MIN_DROPLET_SIZE, MAX_DROPLET_SIZE);
        let direction_sampler = Uniform::new_inclusive(0, 1);
        let rng = rng.as_mut().unwrap();
        let droplets = (0..erosion_chunk_size)
            .map(|_| Droplet {
                position_x: position_sampler.sample(rng),
                position_y: position_sampler.sample(rng),
                radius: radius_sampler.sample(rng),
                sediment: 0.0,
                water: 1.0,
                speed: 0.0,
                direction_x: direction_sampler.sample(rng) as f32,
                direction_y: direction_sampler.sample(rng) as f32,
            })
            .collect_vec();

        erosion_worker.write_slice(ErosionComputeFields::Results, heightmap.data.as_slice());
        erosion_worker.write_slice(ErosionComputeFields::Droplets, droplets.as_slice());

        erosion_worker.execute();
    }
    if *working {
        if *erosion_counter == 0 && erosion_worker.ready() {
            //This will read from the work done the previous frame
            let heightmap_floats = erosion_worker.read_vec(ErosionComputeFields::Results);
            heightmap.data = heightmap_floats;

            heightmap_load_bar.erosion_progress = 1.0;
            *working = false;
            println!(
                "Erosion took: {:?}",
                Instant::now().duration_since(benchmark.unwrap())
            );
        } else if erosion_worker.ready() {
            let map_size = [
                (CHUNK_WORLD_SIZE[0] * HEIGHTMAP_CHUNK_SIZE),
                (CHUNK_WORLD_SIZE[1] * HEIGHTMAP_CHUNK_SIZE),
            ];

            let position_sampler = Uniform::new(0, map_size[0]);
            let radius_sampler = Uniform::new_inclusive(MIN_DROPLET_SIZE, MAX_DROPLET_SIZE);
            let direction_sampler = Uniform::new_inclusive(0, 1);
            let rng = rng.as_mut().unwrap();
            let droplets = (0..erosion_chunk_size)
                .map(|_| Droplet {
                    position_x: position_sampler.sample(rng),
                    position_y: position_sampler.sample(rng),
                    radius: radius_sampler.sample(rng),
                    sediment: 0.0,
                    water: 1.0,
                    speed: 0.0,
                    direction_x: direction_sampler.sample(rng) as f32,
                    direction_y: direction_sampler.sample(rng) as f32,
                })
                .collect_vec();

            erosion_worker.write_slice(ErosionComputeFields::Droplets, droplets.as_slice());

            erosion_worker.execute();

            //This doesn't read from the gpu every frame, since we only need the result for updating the heightmap image, and at the end of the erosion process

            heightmap_load_bar.erosion_progress += 1.0 / erosion_chunks as f32;
            *erosion_counter = erosion_counter.saturating_sub(1);
        }
    }
}
