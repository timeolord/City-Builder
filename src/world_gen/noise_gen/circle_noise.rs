use bevy::math::Vec2;

use noise::{NoiseFn, Seedable};
use rand::{
    distributions::{Distribution, Uniform},
    rngs::StdRng,
    SeedableRng,
};

use crate::{
    utils::math::{normal_curve, AsF32},
    world_gen::{WorldSize, HEIGHTMAP_CHUNK_SIZE},
};
use rand_distr::Normal;

#[derive(Default, Clone)]
pub struct CircleNoise {
    seed: u32,
    amount: u32,
    size: f64,
    map_size: WorldSize,
    std_dev: f64,
    circles: Vec<Circle>,
}

#[derive(Clone, Debug)]
pub struct Circle {
    pub center: [f64; 2],
    pub radius: f64,
    pub std_dev: f64,
}

impl Circle {
    pub fn get(&self, point: [f64; 2]) -> f64 {
        let x = ((Vec2::from_array(point.as_f32()).distance(Vec2::from_array(self.center.as_f32())))
            as f64)
            / self.radius;
        let multiplier = 1.0 / normal_curve(0.0, self.std_dev, 0.0);
        (normal_curve(0.0, self.std_dev, x) * multiplier).clamp(0.0, 1.0)
    }
}

impl CircleNoise {
    pub fn new(seed: u32, amount: u32, size: f64, world_size: WorldSize, std_dev: f64) -> Self {
        let map_size = [
            (world_size[0] * HEIGHTMAP_CHUNK_SIZE) as f64,
            (world_size[1] * HEIGHTMAP_CHUNK_SIZE) as f64,
        ];
        let mut rng = StdRng::seed_from_u64(seed as u64);
        //The x and y positions are uniformally distributed, with 5% margins
        let margins = 0.05;
        let x_sampler = Uniform::new(map_size[0] * margins, map_size[0] - (map_size[0] * margins));
        let y_sampler = Uniform::new(map_size[1] * margins, map_size[1] - (map_size[1] * margins));
        //Circle size is a normal distribution with the standard deviation being 10% of the average size
        let size_sampler = Normal::new(size, size * 0.10).unwrap();

        let circles = (0..amount)
            .map(|_| Circle {
                center: [x_sampler.sample(&mut rng), y_sampler.sample(&mut rng)],
                radius: size_sampler.sample(&mut rng),
                std_dev,
            })
            .collect();

        Self {
            seed,
            amount,
            size,
            circles,
            map_size: world_size,
            std_dev,
        }
    }
}

impl NoiseFn<f64, 2> for CircleNoise {
    fn get(&self, point: [f64; 2]) -> f64 {
        self.circles
            .iter()
            .map(|circle| {
                //println!("{:?}", circle.get(point));
                //println!("{:?}", circle);
                circle.get(point)
            })
            .sum::<f64>()
    }
}

impl Seedable for CircleNoise {
    fn set_seed(self, seed: u32) -> Self {
        if seed == self.seed {
            self
        } else {
            CircleNoise::new(seed, self.amount, self.size, self.map_size, self.std_dev)
        }
    }
    fn seed(&self) -> u32 {
        self.seed
    }
}
