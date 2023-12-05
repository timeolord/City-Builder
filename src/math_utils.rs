use std::{
    borrow::Borrow,
    ops::{Add, Div},
};

use bevy::math::{
    cubic_splines::{CubicBezier, CubicCurve, CubicGenerator},
    Vec2, Vec3,
};
use itertools::Itertools;

pub fn unnormalized_normal_array(a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> Vec3 {
    let normal = (Vec3::from_array(b) - Vec3::from_array(a))
        .cross(Vec3::from_array(c) - Vec3::from_array(a));
    if normal.length().is_sign_negative() {
        -normal
    } else {
        normal
    }
}

pub fn unnormalized_normal_vector(a: Vec3, b: Vec3, c: Vec3) -> Vec3 {
    let normal = (b - a).cross(c - a);
    if normal.length().is_sign_negative() {
        -normal
    } else {
        normal
    }
}

pub fn normal_vector(a: Vec3, b: Vec3, c: Vec3) -> Vec3 {
    unnormalized_normal_vector(a, b, c).normalize()
}

pub trait RoundBy {
    fn round_by(&self, n: Self) -> Self;
}
impl RoundBy for f32 {
    fn round_by(&self, n: Self) -> Self {
        (self / n).round() * n
    }
}
impl RoundBy for f64 {
    fn round_by(&self, n: Self) -> Self {
        (self / n).round() * n
    }
}

pub fn straight_bezier_curve(starting_position: Vec2, ending_position: Vec2) -> CubicCurve<Vec2> {
    CubicBezier::new([[
        starting_position,
        starting_position.lerp(ending_position, 1.0 / 3.0),
        starting_position.lerp(ending_position, 2.0 / 3.0),
        ending_position,
    ]])
    .to_curve()
}
pub trait Arclength {
    fn arclength(&self) -> f32;
}
impl Arclength for CubicCurve<Vec2> {
    fn arclength(&self) -> f32 {
        self.iter_positions(100)
            .tuple_windows()
            .map(|(a, b)| a.distance(b))
            .sum()
    }
}

pub trait Mean {
    fn mean_f32<T, K>(&mut self) -> T
    where
        Self: Iterator<Item = K>,
        K: Borrow<T> + Copy,
        T: Default + Add<T, Output = T> + Div<f32, Output = T> + Copy,
    {
        let mut sum = T::default();
        let mut count: usize = 0;
        for item in &mut *self {
            sum = sum + *item.borrow();
            count += 1;
        }
        sum / count as f32
    }

    fn mean_f64<T, K>(&mut self) -> T
    where
        Self: Iterator<Item = K>,
        K: Borrow<T> + Copy,
        T: Default + Add<T, Output = T> + Div<f64, Output = T> + Copy,
    {
        let mut sum = T::default();
        let mut count: usize = 0;
        for item in &mut *self {
            sum = sum + *item.borrow();
            count += 1;
        }
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
