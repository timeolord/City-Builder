use std::ops::{Index, IndexMut};

use array2d::Array2D;
use bevy::prelude::*;
use image::{DynamicImage, GrayImage};
use itertools::Itertools;
use strum::IntoEnumIterator;

use crate::{
    utils::direction::CardinalDirection,
    utils::math::{AsI32, AsU32},
    world::WorldSize,
};

use super::CHUNK_SIZE;

#[derive(Resource, Clone, Debug)]
pub struct Heightmap {
    data: Array2D<f64>,
}

impl Heightmap {
    pub fn new(size: WorldSize) -> Self {
        Self {
            data: Array2D::filled_with(
                0.0,
                (size[0] * CHUNK_SIZE as u32) as usize,
                (size[1] * CHUNK_SIZE as u32) as usize,
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
    pub fn get_circle(&self, point: [u32; 2], radius: u32) -> Vec<[u32; 2]> {
        let [x, y] = point;
        let radius = radius as i32;
        let side_length = ((radius * 2) + 1) as usize;
        let mut result = Vec::with_capacity(side_length * side_length);
        for dx in -radius..=radius {
            for dy in -radius..=radius {
                let neighbour = [x as i32 + dx, y as i32 + dy];
                if neighbour[0] < self.size()[0] as i32
                    && neighbour[0].is_positive()
                    && neighbour[1] < self.size()[1] as i32
                    && neighbour[1].is_positive()
                {
                    result.push(neighbour.as_u32());
                }
            }
        }
        result
    }
    pub fn as_dynamic_image(self) -> DynamicImage {
        DynamicImage::ImageLuma8(self.clone().into())
    }
    pub fn as_bevy_image(self) -> Image {
        Image::from_dynamic(self.as_dynamic_image(), false)
    }
}

impl From<Heightmap> for GrayImage {
    fn from(heightmap: Heightmap) -> Self {
        let [width, height] = heightmap.size();
        //println!("{}", heightmap.data.as_column_major().iter().max().unwrap());
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
