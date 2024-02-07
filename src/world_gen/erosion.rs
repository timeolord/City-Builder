use bevy::{
    prelude::*,
    tasks::{available_parallelism, block_on, AsyncComputeTaskPool, Task},
    utils::HashMap,
};

use itertools::Itertools;

use rand::{rngs::StdRng, SeedableRng};
use rand_distr::{Distribution, Uniform};
use std::fmt::Debug;

use crate::utils::math::{fast_normal_curve, normal_curve, AsF32, AsU32};

use super::{heightmap::Heightmap, HeightmapLoadBar, WorldGenSettings, CHUNK_SIZE};

#[derive(Event)]
pub struct ErosionEvent;

pub fn erode_heightmap(
    mut heightmap: ResMut<Heightmap>,
    settings: Res<WorldGenSettings>,
    mut heightmap_load_bar: ResMut<HeightmapLoadBar>,
    mut tasks: Local<Vec<Task<HashMap<[u32; 2], f64>>>>,
    mut erosion_counter: Local<u32>,
    mut erosion_event: EventReader<ErosionEvent>,
    mut working: Local<bool>,
) {
    let erosion_chunk_size = 2u32.pow(8);
    let erosion_chunks = settings.erosion_amount;

    if erosion_event.read().count() > 0 {
        *erosion_counter = erosion_chunks;
        heightmap_load_bar.erosion_progress = 0.0;
        tasks.clear();
        *working = true;
    }
    if *working {
        if *erosion_counter == 0 {
            //Blur the heightmap
            for _ in 0..1 {
                let mut new_heightmap = heightmap.clone();
                for x in 0..heightmap.size()[0] {
                    for y in 0..heightmap.size()[1] {
                        let neighbours = heightmap.get_circle([x, y], 1);
                        let mut sum = 0.0;
                        for neighbour in neighbours.iter() {
                            sum += heightmap[*neighbour];
                        }
                        new_heightmap[[x, y]] = sum / neighbours.len() as f64;
                    }
                }
                *heightmap = new_heightmap;
            }

            heightmap_load_bar.erosion_progress = 1.0;
            tasks.clear();
            *working = false;
        } else if tasks.is_empty() {
            let world_size = settings.noise_settings.world_size;
            let seed = settings.noise_settings.seed;
            let map_size = [(world_size[0] * CHUNK_SIZE), (world_size[1] * CHUNK_SIZE)];
            let mut rng = StdRng::seed_from_u64((seed + *erosion_counter) as u64);

            let position_sampler = Uniform::new(0, map_size[0]);
            let radius_sampler = Uniform::new(2u32, 10);
            let thread_pool = AsyncComputeTaskPool::get();

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

            for chunk in positions.chunks(available_parallelism()) {
                let chunk = chunk.to_vec();
                let heightmap = heightmap.clone();
                let task = thread_pool.spawn(async move {
                    let results = chunk.into_iter().map(|(position, radius)| {
                        WaterErosion::new(position, &heightmap, radius).simulate()
                    });
                    let mut result_hashmap = HashMap::new();
                    for result in results {
                        for (position, change) in result {
                            result_hashmap
                                .entry(position)
                                .and_modify(|value| {
                                    *value = (*value + change) / 2.0;
                                })
                                .or_insert(change);
                        }
                    }
                    result_hashmap
                });
                tasks.push(task);
            }
        } else if tasks.iter().all(|task| task.is_finished()) {
            let mut result_hashmap = HashMap::new();
            for task in &mut tasks {
                let result = block_on(task);
                for (position, change) in result {
                    result_hashmap
                        .entry(position)
                        .and_modify(|value| {
                            *value += change;
                        })
                        .or_insert(change);
                }
            }
            for (position, change) in result_hashmap.iter() {
                heightmap[*position] += change;
                //Checks for any weird artifacts
                if heightmap[*position] >= 1.0 {
                    println!("{:?} {:?}", position, change);
                }
            }
            tasks.clear();
            heightmap_load_bar.erosion_progress += 1.0 / erosion_chunks as f32;
            *erosion_counter = erosion_counter.saturating_sub(1);
        }
    }
}

const MAX_EROSION_STEPS: u32 = 1000;
struct WaterErosion<'a> {
    position: [u32; 2],
    sediment: f64,
    water: f64,
    speed: f64,
    direction: Vec2,
    moves: HashMap<[u32; 2], f64>,
    heightmap: &'a Heightmap,
    radius: u32,
}

impl Debug for WaterErosion<'_> {
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
    pub fn new(position: [u32; 2], heightmap: &'a Heightmap, radius: u32) -> Self {
        Self {
            position,
            sediment: 0.0,
            water: 1.0,
            speed: 0.0,
            direction: Vec2::new(0.0, 0.0),
            moves: HashMap::new(),
            heightmap,
            radius,
        }
    }
    fn deposit(&mut self, amount: f64) {
        //Deposts sediment in a circle to prevent peaks
        let std_dev = self.radius as f64 / 2.0;
        let tiles = self.heightmap.get_circle(self.position, self.radius);
        for tile in tiles {
            let distance = Vec2::from_array(self.position.as_f32())
                .distance(Vec2::from_array(tile.as_f32())) as f64;
            let deposition_amount = amount * fast_normal_curve(0.0, std_dev, distance);
            /* let deposition_amount = amount * self.normal_curve[distance as usize]; */
            /* let deposition_amount = amount * (1.0 / (distance + 1.0)) * 0.2; */
            /* * (((self.radius as f64 - distance) / self.radius as f64)
                    * (2.0 / self.radius as f64)) */
            self.moves
                .entry(tile)
                .and_modify(|x| *x += deposition_amount)
                .or_insert(deposition_amount);
        }
    }
    fn erode(&mut self, amount: f64) {
        self.deposit(-amount);
        /* let neighbours = self.heightmap.neighbours(self.position).collect_vec();
        let deposition_amount = (amount * 0.5) / (neighbours.len() as f64);
        self.moves
            .entry(self.position)
            .and_modify(|x| *x -= amount)
            .or_insert(-amount);
        for neighbour in neighbours {
            self.moves
                .entry(neighbour)
                .and_modify(|x| *x -= deposition_amount)
                .or_insert(-deposition_amount);
        } */
    }
    fn read(&self, position: [u32; 2]) -> f64 {
        self.heightmap[position] + self.moves.get(&position).unwrap_or(&0.0)
    }
    pub fn simulate(mut self) -> HashMap<[u32; 2], f64> {
        let debug = false;
        let erosion_speed: f64 = 0.9;
        let gravity = 30.0;
        let deposition_speed: f64 = 0.5;
        let water_evaporation_speed: f64 = 0.0001;
        let minimum_slope = 0.01;
        let direction_inertia = 3.0;
        let _carry_capacity_modifier = 1.0;

        for _ in 0..MAX_EROSION_STEPS {
            let position = self.position;

            //Calculate gradient
            let neighbours = self.heightmap.get_circle(position, self.radius);
            for neighbour in neighbours {
                if neighbour == position {
                    continue;
                }
                let direction: Vec2 =
                    Vec2::from_array(self.position.as_f32()) - Vec2::from_array(neighbour.as_f32());
                let height_difference = self.read(position) - self.read(neighbour);
                self.direction +=
                    (direction.normalize() * -(height_difference as f32) * gravity as f32)
                        * direction_inertia;
            }
            let next_position = (Vec2::from_array(self.position.as_f32())
                + (self.direction.normalize() * self.radius as f32))
                .round()
                .to_array()
                .as_u32();

            //If the next position is out of bounds, stop the erosion
            if self.heightmap.get(next_position).is_none() {
                if debug {
                    println!("Out of bounds");
                }
                return self.moves;
            }

            //Water evaporation
            self.water *= 1.0 - water_evaporation_speed;

            let height_difference = self.read(position) - self.read(next_position);
            self.speed += height_difference * gravity;
            self.direction = self.direction.normalize() * self.speed as f32;

            let carry_capacity = height_difference.max(minimum_slope)
                * self.speed
                * self.water
                * self.radius as f64
                /* * carry_capacity_modifier */;

            //Deposition if evaporated or speed is negative
            if self.water < 0.1 || self.speed.is_sign_negative() {
                let old_radius = self.radius;
                self.radius = 25;
                self.deposit(self.sediment);
                self.radius = old_radius;
                if debug {
                    println!("Water Evaporated");
                }
                return self.moves;
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
        }
        return self.moves;
    }
}
