use std::{borrow::Borrow, ops::Add};

use bevy::math::{cubic_splines::CubicCurve, Vec2, Vec3};
use itertools::Itertools;
use num_traits::AsPrimitive;

use crate::world::{heightmap::HeightmapsResource, road::Road};

pub fn unnormalized_normal_vector(a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> Vec3 {
    (Vec3::from_array(b) - Vec3::from_array(a)).cross(Vec3::from_array(c) - Vec3::from_array(a))
}

pub fn average_vectors<const N: usize>(list: [Vec3; N]) -> Vec3 {
    let mut sum = Vec3::new(0.0, 0.0, 0.0);
    for vector in list {
        sum += vector;
    }
    sum / N as f32
}

pub fn round_to(x: f32, n: f32) -> f32 {
    (x / n).round() * n
}

pub trait Mean: Iterator {
    fn mean_f32<T, K>(&mut self) -> f32
    where
        Self: Iterator<Item = K>,
        K: Borrow<T> + Copy,
        T: Default + Add<T, Output = T> + AsPrimitive<f32>,
    {
        let mut sum = T::default();
        let mut count: usize = 0;
        for item in self.into_iter() {
            sum = sum + *item.borrow();
            count += 1;
        }
        let sum: f32 = sum.as_();
        sum / count as f32
    }

    fn mean_f64<T, K>(&mut self) -> f64
    where
        Self: Iterator<Item = K>,
        K: Borrow<T> + Copy,
        T: Default + Add<T, Output = T> + AsPrimitive<f64>,
    {
        let mut sum = T::default();
        let mut count: usize = 0;
        for item in self.into_iter() {
            sum = sum + *item.borrow();
            count += 1;
        }
        let sum: f64 = sum.as_();
        sum / count as f64
    }
}
impl<T: ?Sized> Mean for T where T: Iterator {}

pub trait AsF32<T, const N: usize> {
    fn as_f32(&self) -> [f32; N];
}
impl<T: num_traits::cast::AsPrimitive<f32>, const N: usize> AsF32<T, N> for [T; N] {
    fn as_f32(&self) -> [f32; N] {
        let mut array = [0.0; N];
        for (i, item) in self.iter().enumerate() {
            array[i] = (*item).as_();
        }
        array
    }
}
pub trait AsU32<T, const N: usize> {
    fn as_u32(&self) -> [u32; N];
}
impl<T: num_traits::cast::AsPrimitive<u32>, const N: usize> AsU32<T, N> for [T; N] {
    fn as_u32(&self) -> [u32; N] {
        let mut array = [0; N];
        for (i, item) in self.iter().enumerate() {
            array[i] = (*item).as_();
        }
        array
    }
}
pub trait AsI32<T, const N: usize> {
    fn as_i32(&self) -> [i32; N];
}
impl<T: num_traits::cast::AsPrimitive<i32>, const N: usize> AsI32<T, N> for [T; N] {
    fn as_i32(&self) -> [i32; N] {
        let mut array = [0; N];
        for (i, item) in self.iter().enumerate() {
            array[i] = (*item).as_();
        }
        array
    }
}
