use std::ops::{Index, IndexMut};

use crate::{
    utils::direction::CardinalDirection,
    utils::math::{AsI32, AsU32},
    world::WorldSize,
};
use array2d::Array2D;
use bevy::prelude::*;
use image::{DynamicImage, GrayImage};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;

use super::HEIGHTMAP_CHUNK_SIZE;

#[derive(Resource, Clone, Debug, Serialize, Deserialize)]
pub struct Heightmap {
    data: Array2D<f64>,
}

impl Heightmap {
    pub fn new(size: WorldSize) -> Self {
        Self {
            data: Array2D::filled_with(
                0.0,
                (size[0] * HEIGHTMAP_CHUNK_SIZE as u32) as usize,
                (size[1] * HEIGHTMAP_CHUNK_SIZE as u32) as usize,
            ),
        }
    }
    pub fn get(&self, point: [u32; 2]) -> Option<f64> {
        self.data.get(point[0] as usize, point[1] as usize).copied()
    }
    pub fn size(&self) -> WorldSize {
        [self.data.num_rows() as u32, self.data.num_columns() as u32]
    }
    pub fn neighbours(&self, point: [u32; 2]) -> impl Iterator<Item = [u32; 2]> + '_ {
        CardinalDirection::iter().filter_map(move |dir| {
            let neighbour = point.as_i32() + dir;
            if neighbour[0] < self.size()[0] as i32
                && neighbour[0].is_positive()
                && neighbour[1] < self.size()[1] as i32
                && neighbour[1].is_positive()
            {
                Some(neighbour.as_u32())
            } else {
                None
            }
        })
    }
    pub fn get_circle(&self, point: [u32; 2], radius: u32) -> HeightmapCircle {
        HeightmapCircle {
            center: point.as_i32(),
            heightmap_size: self.size(),
            radius,
            dx: -(radius as i32),
            dy: -(radius as i32),
        }
    }
    pub fn as_dynamic_image(self) -> DynamicImage {
        DynamicImage::ImageLuma8(self.clone().into())
    }
    pub fn as_bevy_image(self) -> Image {
        Image::from_dynamic(self.as_dynamic_image(), false)
    }
}

pub struct HeightmapCircle {
    pub center: [i32; 2],
    pub heightmap_size: WorldSize,
    pub radius: u32,
    pub dx: i32,
    pub dy: i32,
}
impl Iterator for HeightmapCircle {
    type Item = [u32; 2];

    fn next(&mut self) -> Option<Self::Item> {
        let [x, y] = self.center;

        loop {
            if self.dy > self.radius as i32 {
                return None;
            }
            let neighbour = [x + self.dx, y + self.dy];
            self.dx += 1;
            if self.dx > self.radius as i32 {
                self.dx = -(self.radius as i32);
                self.dy += 1;
            }
            if neighbour[0] < self.heightmap_size[0] as i32
                && neighbour[0].is_positive()
                && neighbour[1] < self.heightmap_size[1] as i32
                && neighbour[1].is_positive()
            {
                return Some(neighbour.as_u32());
            }
        }
    }
}

impl From<Heightmap> for GrayImage {
    fn from(heightmap: Heightmap) -> Self {
        let [width, height] = heightmap.size();
        GrayImage::from_raw(
            width,
            height,
            heightmap
                .data
                .as_column_major()
                .iter()
                .map(|&x| (x * 255.0) as u8)
                .collect_vec(),
        )
        .expect("Failed to convert heightmap to image")
    }
}

impl Index<[u32; 2]> for Heightmap {
    type Output = f64;

    fn index(&self, index: [u32; 2]) -> &Self::Output {
        &self.data[(index[0] as usize, index[1] as usize)]
    }
}
impl IndexMut<[u32; 2]> for Heightmap {
    fn index_mut(&mut self, index: [u32; 2]) -> &mut Self::Output {
        &mut self.data[(index[0] as usize, index[1] as usize)]
    }
}
