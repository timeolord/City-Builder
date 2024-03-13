use bevy::{
    prelude::*,
    render::{
        render_resource::{
            Buffer, BufferDescriptor, BufferInitDescriptor, BufferUsages, ShaderType,
        },
        renderer::RenderDevice,
    },
};

use image::EncodableLayout;
use itertools::Itertools;

use rand::{rngs::StdRng, SeedableRng};
use rand_distr::{Distribution, Uniform};
use std::{
    fmt::Debug,
    mem::swap,
    sync::{Arc, RwLock},
};

use crate::{
    shaders::{ComputeShaderResource, ComputeShaderRunType},
    utils::math::{fast_normal_approx, AsF32, AsU32},
};

use super::{
    heightmap::{Heightmap, HeightmapImage},
    HeightmapLoadBar, WorldSettings, HEIGHTMAP_CHUNK_SIZE,
};
use bevy::render::extract_resource::ExtractResource;
use bevy::render::render_resource::AsBindGroup;
use std::time::Instant;

pub const MAX_DROPLET_SIZE: u32 = 12;
pub const MIN_DROPLET_SIZE: u32 = 2;
pub const WORKGROUP_SIZE: u64 = 64;

#[derive(ExtractResource, AsBindGroup, Resource, Clone, Debug)]
pub struct ComputeErosion {
    #[storage(0, visibility(compute))]
    pub droplets: Vec<Droplet>,
    #[storage(1, visibility(compute), buffer)]
    pub results: Buffer,
    #[uniform(2, visibility(compute))]
    pub size: UVec2,
    #[uniform(3, visibility(compute))]
    pub world_size: UVec2,
    pub result_bytes: Arc<RwLock<Vec<u8>>>,
    pub dispatch_size: [u32; 3],
    pub run_condition: Arc<RwLock<ComputeShaderRunType>>,
}
impl ComputeErosion {
    pub fn stop_and_clean_up(&mut self) {
        self.cleanup();
        self.droplets = vec![];
    }
}

impl ComputeShaderResource for ComputeErosion {
    fn result_buffer(&self) -> &Buffer {
        &self.results
    }
    fn mapped_bytes(&self) -> &Arc<RwLock<Vec<u8>>> {
        &self.result_bytes
    }
    fn dispatch_size(&self) -> [u32; 3] {
        self.dispatch_size
    }
    fn run_condition(&self) -> &Arc<RwLock<ComputeShaderRunType>> {
        &self.run_condition
    }
}

#[derive(Event)]
pub struct ErosionEvent;

pub fn gpu_erode_heightmap(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    mut heightmap: ResMut<Heightmap>,
    heightmap_image: Res<HeightmapImage>,
    image_assets: ResMut<Assets<Image>>,
    settings: Res<WorldSettings>,
    mut heightmap_load_bar: ResMut<HeightmapLoadBar>,
    mut erosion_counter: Local<u32>,
    mut erosion_event: EventReader<ErosionEvent>,
    mut working: Local<bool>,
    mut benchmark: Local<Option<Instant>>,
    mut compute_erosion: Option<ResMut<ComputeErosion>>,
    mut heightmap_bytes: Local<Vec<u8>>,
    mut heightmap_floats: Local<Vec<f32>>,
    mut rng: Local<Option<StdRng>>,
) {
    let erosion_chunks = settings.erosion_amount;
    let dispatch_size = 16;
    let erosion_chunk_size = dispatch_size as u64 * WORKGROUP_SIZE;

    if erosion_event.read().count() > 0 {
        if let Some(compute_erosion) = compute_erosion.as_mut() {
            compute_erosion.stop_and_clean_up();
        }
        let image_length = image_assets
            .get(heightmap_image.image.clone())
            .unwrap()
            .data
            .len();
        *heightmap_bytes = vec![0u8; image_length];
        *heightmap_floats = vec![0.0; image_length / 4];
        //Initalize Erosion Resource
        let results = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("Erosion Result Buffer"),
            contents: heightmap.data.as_bytes(),
            usage: BufferUsages::COPY_SRC | BufferUsages::STORAGE,
        });
        let result_bytes: Arc<RwLock<Vec<u8>>> = Arc::new(RwLock::new(vec![0u8; image_length]));
        let droplets = vec![Droplet::default(); dispatch_size as usize * WORKGROUP_SIZE as usize];
        commands.insert_resource(ComputeErosion {
            droplets,
            results,
            size: heightmap.size().into(),
            world_size: settings.tile_world_size().into(),
            result_bytes,
            dispatch_size: [dispatch_size, 1, 1],
            run_condition: Arc::new(RwLock::new(ComputeShaderRunType::EveryFrame)),
        });
        *rng = Some(StdRng::seed_from_u64(settings.noise_settings.seed as u64));

        *erosion_counter = erosion_chunks;
        heightmap_load_bar.erosion_progress = 0.0;
        *working = true;
        *benchmark = Some(Instant::now());
        //Wait 1 frame for the compute erosion resource to be created
    } else if *working {
        if *erosion_counter == 0 {
            //This will read from the work done the previous frame
            for (index, byte) in compute_erosion
                .as_ref()
                .unwrap()
                .result_bytes
                .read()
                .unwrap()
                .chunks_exact(4)
                .enumerate()
            {
                heightmap_floats[index] = f32::from_ne_bytes([byte[0], byte[1], byte[2], byte[3]]);
            }
            swap(&mut *heightmap_floats, &mut heightmap.data);

            //Smooth the heightmap
            for _ in 0..2 {
                let mut new_heightmap = heightmap.clone();
                for x in 0..heightmap.size()[0] {
                    for y in 0..heightmap.size()[1] {
                        let neighbours = heightmap.get_circle([x, y], 1);
                        let mut sum = 0.0;
                        let mut length = 0;
                        for neighbour in neighbours {
                            sum += heightmap[neighbour];
                            length += 1;
                        }
                        new_heightmap[[x, y]] = sum / length as f32;
                    }
                }
                *heightmap = new_heightmap;
            }

            //Blur the heightmap in parts where it has erosion artifacts
            for _ in 0..10 {
                let radius = 2;
                let mut new_heightmap = heightmap.clone();
                for x in 0..(heightmap.size()[0] / radius) {
                    for y in 0..(heightmap.size()[1] / radius) {
                        let neighbours = heightmap.get_circle([x, y], radius);
                        let mut max_height_delta = 0.0f32;
                        for neighbour in neighbours.clone() {
                            let height_delta = (heightmap[[x, y]] - heightmap[neighbour]).abs();
                            max_height_delta = max_height_delta.max(height_delta)
                        }
                        if max_height_delta > 0.1 {
                            let mut sum = 0.0;
                            let mut length = 0;
                            for neighbour in neighbours {
                                sum += heightmap[neighbour];
                                length += 1;
                            }
                            new_heightmap[[x, y]] = sum / length as f32;
                        }
                    }
                }
                *heightmap = new_heightmap;
            }
            heightmap_load_bar.erosion_progress = 1.0;
            *working = false;
            compute_erosion.as_mut().unwrap().stop_and_clean_up();
            *heightmap_bytes = vec![];
            *heightmap_floats = vec![];
            println!(
                "Erosion took: {:?}",
                Instant::now().duration_since(benchmark.unwrap())
            );
        } else {
            let world_size = settings.noise_settings.world_size;
            let map_size = [
                (world_size[0] * HEIGHTMAP_CHUNK_SIZE),
                (world_size[1] * HEIGHTMAP_CHUNK_SIZE),
            ];

            let position_sampler = Uniform::new(0, map_size[0]);
            let radius_sampler = Uniform::new_inclusive(MIN_DROPLET_SIZE, MAX_DROPLET_SIZE);
            let direction_sampler = Uniform::new_inclusive(0, 1);
            let droplets = &mut compute_erosion.as_mut().unwrap().droplets;
            let rng = rng.as_mut().unwrap();

            for index in 0..erosion_chunk_size as usize {
                let droplet = Droplet {
                    position_x: position_sampler.sample(rng),
                    position_y: position_sampler.sample(rng),
                    radius: radius_sampler.sample(rng),
                    sediment: 0.0,
                    water: 1.0,
                    speed: 0.0,
                    direction_x: direction_sampler.sample(rng) as f32,
                    direction_y: direction_sampler.sample(rng) as f32,
                };
                droplets[index] = droplet;
            }

            //This doesn't read from the gpu every frame, since we only need the result for updating the heightmap image, and at the end of the erosion process

            heightmap_load_bar.erosion_progress += 1.0 / erosion_chunks as f32;
            *erosion_counter = erosion_counter.saturating_sub(1);
        }
    }
}

#[derive(Debug, Clone, ShaderType, Default)]
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

pub fn erode_heightmap(
    mut heightmap: ResMut<Heightmap>,
    settings: Res<WorldSettings>,
    mut heightmap_load_bar: ResMut<HeightmapLoadBar>,
    mut erosion_counter: Local<u32>,
    mut erosion_event: EventReader<ErosionEvent>,
    mut working: Local<bool>,
    mut benchmark: Local<Option<Instant>>,
) {
    let erosion_chunk_size = 2u32.pow(4);
    /* let erosion_chunk_size = 1u32; */
    let erosion_chunks = settings.erosion_amount;
    let max_runtime = 1.0 / 30.0;
    let start_time = Instant::now();

    if erosion_event.read().count() > 0 {
        *erosion_counter = erosion_chunks;
        heightmap_load_bar.erosion_progress = 0.0;
        *working = true;
        *benchmark = Some(Instant::now());
    }
    if *working {
        while (Instant::now() - start_time).as_secs_f64() < max_runtime {
            if *erosion_counter == 0 {
                //Blur the heightmap
                for _ in 0..1 {
                    let mut new_heightmap = heightmap.clone();
                    for x in 0..heightmap.size()[0] {
                        for y in 0..heightmap.size()[1] {
                            let neighbours = heightmap.get_circle([x, y], 1);
                            let mut sum = 0.0;
                            let mut length = 0;
                            for neighbour in neighbours {
                                sum += heightmap[neighbour];
                                length += 1;
                            }
                            new_heightmap[[x, y]] = sum / length as f32;
                        }
                    }
                    *heightmap = new_heightmap;
                }
                heightmap_load_bar.erosion_progress = 1.0;
                *working = false;
                println!(
                    "Erosion took: {:?}",
                    Instant::now().duration_since(benchmark.unwrap())
                );
            } else {
                let world_size = settings.noise_settings.world_size;
                let seed = settings.noise_settings.seed;
                let map_size = [
                    (world_size[0] * HEIGHTMAP_CHUNK_SIZE),
                    (world_size[1] * HEIGHTMAP_CHUNK_SIZE),
                ];
                let mut rng = StdRng::seed_from_u64((seed + *erosion_counter) as u64);

                let position_sampler = Uniform::new(0, map_size[0]);
                let radius_sampler = Uniform::new(MIN_DROPLET_SIZE, MAX_DROPLET_SIZE);

                let positions = (0..erosion_chunk_size)
                    .map(|_| {
                        (
                            [
                                position_sampler.sample(&mut rng),
                                position_sampler.sample(&mut rng),
                            ],
                            radius_sampler.sample(&mut rng),
                        )
                    })
                    .collect_vec();

                positions.into_iter().for_each(|(position, radius)| {
                    WaterErosion::new(position, &mut heightmap, radius).simulate()
                });
                heightmap_load_bar.erosion_progress += 1.0 / erosion_chunks as f32;
                *erosion_counter = erosion_counter.saturating_sub(1);
            }
            #[cfg(unix)]
            {
                coz::progress!("Erode Heightmap");
            }
        }
    }
}

const MAX_EROSION_STEPS: u32 = 500;
struct WaterErosion<'a> {
    position: [u32; 2],
    sediment: f32,
    water: f32,
    speed: f32,
    direction: Vec2,
    heightmap: &'a mut Heightmap,
    radius: u32,
}

impl<'a> Debug for WaterErosion<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WaterErosion")
            .field("position", &self.position)
            .field("sediment", &self.sediment)
            .field("water", &self.water)
            .field("speed", &self.speed)
            .field("direction", &self.direction)
            .field("raidus", &self.radius)
            .finish()
    }
}

impl<'a> WaterErosion<'a> {
    pub fn new(position: [u32; 2], heightmap: &'a mut Heightmap, radius: u32) -> Self {
        Self {
            position,
            sediment: 0.0,
            water: 1.0,
            speed: 0.0,
            direction: Vec2::new(0.0, 0.0),
            heightmap,
            radius,
        }
    }
    fn deposit(&mut self, amount: f32) {
        //Deposts sediment in a circle to prevent peaks
        //NOTE: After careful benchmarking, using raw for loops is faster than using iterators from get_circle.
        let [x, y] = self.position;
        let radius = self.radius as i32;
        for dx in -radius..=radius {
            for dy in -radius..=radius {
                let neighbour = [x as i32 + dx, y as i32 + dy];
                if neighbour[0] < self.heightmap.size()[0] as i32
                    && neighbour[0].is_positive()
                    && neighbour[1] < self.heightmap.size()[1] as i32
                    && neighbour[1].is_positive()
                {
                    let distance = Vec2::from_array(self.position.as_f32())
                        .distance(Vec2::from_array(neighbour.as_f32()))
                        as f64;
                    let deposition_amount =
                        amount * fast_normal_approx(self.radius as f64, distance) as f32;
                    self.heightmap[neighbour.as_u32()] += deposition_amount;
                }
            }
        }
    }
    fn erode(&mut self, amount: f32) {
        self.deposit(-amount);
    }
    fn read(&self, position: [u32; 2]) -> f32 {
        self.heightmap[position]
    }
    pub fn simulate(mut self) {
        let debug = false;
        let erosion_speed: f32 = 0.9;
        let gravity = 30.0;
        let deposition_speed: f32 = 0.2;
        let water_evaporation_speed: f32 = 0.0001;
        let minimum_slope = 0.01;
        let direction_inertia = 3.0;
        let carry_capacity_modifier = 2.0;

        for _ in 0..MAX_EROSION_STEPS {
            let position = self.position;

            //Calculate gradient
            let neighbours = self.heightmap.get_circle(position, self.radius);
            for neighbour in neighbours.clone() {
                if neighbour == position {
                    continue;
                }
                let direction: Vec2 =
                    Vec2::from_array(self.position.as_f32()) - Vec2::from_array(neighbour.as_f32());
                let height_difference = self.read(position) - self.read(neighbour);
                self.direction +=
                    (direction * -(height_difference as f32) * gravity as f32) * direction_inertia;
            }
            let normalized_direction = self.direction.try_normalize();
            if normalized_direction.is_none() {
                return;
            } else {
                self.direction = normalized_direction.unwrap();
            }
            let next_position = (Vec2::from_array(self.position.as_f32())
                + (self.direction * self.radius as f32))
                .round()
                .to_array()
                .as_u32();

            //If the next position is out of bounds, stop the erosion
            if self.heightmap.get(next_position).is_none() {
                if debug {
                    println!("Out of bounds");
                }
                return;
            }

            //Water evaporation
            self.water *= 1.0 - water_evaporation_speed;

            let height_difference = self.read(position) - self.read(next_position);
            self.speed += height_difference * gravity;
            self.direction = self.direction * self.speed as f32;

            let carry_capacity = height_difference.max(minimum_slope)
                * self.speed
                * self.water
                * self.radius as f32
                * carry_capacity_modifier;

            //Deposition if evaporated or speed is negative
            if self.water < 0.1 || self.speed.is_sign_negative() {
                self.radius = 50;
                self.deposit(self.sediment * deposition_speed);
                if debug {
                    println!("Water Evaporated");
                }
                return;
            }
            //Deposition if sediment is too high
            if self.sediment > carry_capacity || height_difference.is_sign_negative() {
                let deposit_amount;
                if height_difference.is_sign_negative() {
                    //Fills in pits
                    deposit_amount = (-height_difference).min(self.sediment);
                } else {
                    /* let sediment_delta = self.sediment - carry_capacity; */
                    let sediment_delta = self.sediment;
                    deposit_amount = deposition_speed * sediment_delta;
                }
                self.deposit(deposit_amount);
                self.sediment -= deposit_amount;
                if debug {
                    println!("Deposited: {:?}", deposit_amount);
                }
            }
            //Erosion
            else {
                let sediment_delta = carry_capacity - self.sediment;
                let erosion_amount = erosion_speed * sediment_delta;
                let erosion_amount = erosion_amount.min(height_difference);
                if erosion_amount.is_sign_negative() {
                    println!("{:?}", erosion_amount);
                }
                self.erode(erosion_amount);
                self.sediment += erosion_amount;
                if debug {
                    println!("Eroded: {:?}", erosion_amount);
                }
            }
            self.position = next_position;
            #[cfg(unix)]
            {
                coz::progress!("Erosion Step");
            }
        }
        self.radius = 50;
        self.deposit(self.sediment * deposition_speed);
        return;
    }
}
