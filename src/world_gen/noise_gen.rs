mod circle_noise;

use itertools::Itertools;
use noise::*;
use ordered_float::NotNan;
use serde::{Deserialize, Serialize};

use crate::world::WorldSize;

use self::circle_noise::CircleNoise;

#[derive(Clone, Copy)]
pub struct NoiseGenerator<Noise> {
    noise: Noise,
}

impl<Noise> NoiseGenerator<Noise> {
    pub fn new(noise: Noise) -> Self {
        Self { noise }
    }
}

impl<Noise> NoiseFunction for NoiseGenerator<Noise>
where
    Noise: NoiseFn<f64, 2>,
{
    fn get(&self, index: [u32; 2]) -> f64 {
        let [x, y] = index;
        //Does this need to be clamped?
        (self.noise.get([x as f64, y as f64]).clamp(-1.0, 1.0) + 1.0) / 2.0
    }
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct NoiseSettings {
    pub seed: u32,
    pub mountain_amount: u32,
    pub mountain_size: f64,
    pub hilliness: f64,
    pub world_size: WorldSize,
}

impl NoiseSettings {
    pub fn new(world_size: WorldSize) -> Self {
        Self {
            world_size,
            ..Default::default()
        }
    }
}

impl PartialEq for NoiseSettings {
    fn eq(&self, other: &Self) -> bool {
        self.seed == other.seed
            && self.mountain_amount == other.mountain_amount
            && NotNan::new(self.mountain_size) == NotNan::new(other.mountain_size)
            && NotNan::new(self.hilliness) == NotNan::new(other.hilliness)
    }
}
impl Eq for NoiseSettings {}

impl Default for NoiseSettings {
    fn default() -> Self {
        Self {
            seed: 0,
            mountain_amount: 1,
            mountain_size: 100.0,
            hilliness: 0.5,
            world_size: [0, 0],
        }
    }
}

pub trait NoiseFunction {
    fn get(&self, index: [u32; 2]) -> f64;
}

pub fn noise_function(settings: NoiseSettings) -> impl NoiseFunction {
    let seed = settings.seed;
    let hilliness = settings.hilliness;
    let mountain_size = settings.mountain_size;

    let octaves: usize = 4;
    let sources = (0..octaves)
        .into_iter()
        .map(|i| Perlin::new(seed.wrapping_add(i as u32)))
        .collect_vec();

    let mountain_noise = RidgedMulti::new(seed)
        .set_octaves(octaves)
        .set_sources(sources.clone());

    let mountain_noise = ScalePoint::new(mountain_noise).set_scale((1.0 / (mountain_size)) * 0.4);

    let base_terrain_noise = Fbm::new(seed)
        .set_octaves(octaves)
        .set_sources(sources.clone());
    let base_terrain_noise = ScalePoint::new(base_terrain_noise).set_scale(0.001);
    let base_terrain_noise = ScaleBias::new(base_terrain_noise)
        .set_scale(hilliness)
        .set_bias(-0.7);

    let mountain_masks = CircleNoise::new(
        seed,
        settings.mountain_amount,
        settings.mountain_size,
        settings.world_size,
        1.0,
    );

    let noise = Blend::new(base_terrain_noise, mountain_noise, mountain_masks.clone());

    NoiseGenerator::new(noise)
}
