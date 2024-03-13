use std::{
    ops::{Index, IndexMut},
    sync::{Arc, RwLock},
};

use crate::{
    shaders::{ComputeShaderResource, ComputeShaderRunType},
    utils::{
        direction::CardinalDirection,
        math::{bilinear_interpolation, AsI32, AsU32},
    },
    world::WorldSize,
};
use array2d::Array2D;
use bevy::{
    prelude::*,
    render::{
        extract_resource::ExtractResource,
        render_asset::RenderAssetUsages,
        render_resource::{AsBindGroup, Buffer, TextureUsages},
    },
};
use image::{DynamicImage, RgbaImage};
use itertools::Itertools;
use num::Integer;
use num_traits::AsPrimitive;
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;

use super::{mesh_gen::WORLD_HEIGHT_SCALE, CHUNK_SIZE, HEIGHTMAP_CHUNK_SIZE};

#[derive(Resource, Clone, Debug, Serialize, Deserialize)]
pub struct Heightmap {
    pub data: Vec<f32>,
    pub tree_density: Array2D<f64>,
    size: WorldSize,
}

#[derive(ExtractResource, Resource, Clone, Debug)]
pub struct HeightmapImage {
    pub image: Handle<Image>,
    pub size: UVec2,
}

impl Heightmap {
    pub fn new(size: WorldSize) -> Self {
        /* Self {
            data: Array2D::filled_with(
                0.0,
                (size[0] * HEIGHTMAP_CHUNK_SIZE as u32) as usize,
                (size[1] * HEIGHTMAP_CHUNK_SIZE as u32) as usize,
            ),
            tree_density: Array2D::filled_with(
                0.5,
                (size[0] * CHUNK_SIZE as u32) as usize,
                (size[1] * CHUNK_SIZE as u32) as usize,
            ),
        } */
        Self {
            data: vec![0.0; (size[0] * HEIGHTMAP_CHUNK_SIZE * size[1] * HEIGHTMAP_CHUNK_SIZE) as usize],
            tree_density: Array2D::filled_with(
                0.5,
                (size[0] * CHUNK_SIZE as u32) as usize,
                (size[1] * CHUNK_SIZE as u32) as usize,
            ),
            size: [size[0] * HEIGHTMAP_CHUNK_SIZE, size[1] * HEIGHTMAP_CHUNK_SIZE],
        }
    }
    pub fn get<N: Integer + AsPrimitive<usize>, T: Into<[N; 2]>>(&self, point: T) -> Option<f32> {
        let point = point.into();
        self.data.get(point[0].as_() * self.size[1] as usize + point[1].as_()).copied()
    }
    pub fn size(&self) -> WorldSize {
        [self.size[0], self.size[1]]
    }
    pub fn tree_density(&self, point: [u32; 2]) -> f64 {
        self.tree_density[(point[0] as usize, point[1] as usize)]
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
        DynamicImage::ImageRgba8(self.clone().into())
    }
    pub fn as_bevy_image(self) -> Image {
        let mut image = Image::from_dynamic(
            self.as_dynamic_image(),
            false,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        );
        image.texture_descriptor.usage = TextureUsages::COPY_DST
            | TextureUsages::STORAGE_BINDING
            | TextureUsages::TEXTURE_BINDING;
        image
    }
    pub fn interpolate_height(&self, position: Vec2) -> f32 {
        let fractional_position = position.xy().fract();
        let integer_position = position.floor().as_uvec2();
        let heights = vec![
            self[integer_position],
            self[integer_position + UVec2::from_array([1, 0])],
            self[integer_position + UVec2::from_array([0, 1])],
            self[integer_position + UVec2::from_array([1, 1])],
        ];
        let x = bilinear_interpolation(
            [heights[0], heights[1]],
            [heights[2], heights[3]],
            [fractional_position.x, fractional_position.y],
        );
        x * WORLD_HEIGHT_SCALE
    }
}

#[derive(Debug, Clone)]
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

impl From<Heightmap> for RgbaImage {
    fn from(heightmap: Heightmap) -> Self {
        let [width, height] = heightmap.size();
        RgbaImage::from_raw(
            width,
            height,
            heightmap
                .data
                .iter()
                .map(|&x| [(x * 255.0) as u8, (x * 255.0) as u8, (x * 255.0) as u8, 255])
                .flatten()
                .collect_vec(),
        )
        .expect("Failed to convert heightmap to image")
    }
}
impl Index<UVec2> for Heightmap {
    type Output = f32;

    fn index(&self, index: UVec2) -> &Self::Output {
        &self[index.to_array()]
    }
}
impl IndexMut<UVec2> for Heightmap {
    fn index_mut(&mut self, index: UVec2) -> &mut Self::Output {
        &mut self[index.to_array()]
    }
}
impl Index<[u32; 2]> for Heightmap {
    type Output = f32;

    fn index(&self, index: [u32; 2]) -> &Self::Output {
        &self.data[index[0] as usize * self.size[1] as usize + index[1] as usize]
    }
}
impl IndexMut<[u32; 2]> for Heightmap {
    fn index_mut(&mut self, index: [u32; 2]) -> &mut Self::Output {
        &mut self.data[index[0] as usize * self.size[1] as usize + index[1] as usize]
    }
}
