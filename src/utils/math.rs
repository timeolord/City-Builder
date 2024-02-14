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
pub fn round_even_up(n: u32) -> u32 {
    match n {
        even if even % 2 == 0 => even + 1,
        odd => odd,
    }
}

#[inline(always)]
pub fn fast_normal_curve(mean: f64, std_dev: f64, x: f64) -> f64 {
    let a = 1.0 / (std_dev * (2.0 * std::f64::consts::PI).sqrt());
    let b = -0.5 * ((x - mean) / std_dev).powi(2);
    a * fast_math::exp_raw(b as f32) as f64
}

#[inline(always)]
pub fn normal_curve(mean: f64, std_dev: f64, x: f64) -> f64 {
    let a = 1.0 / (std_dev * (2.0 * std::f64::consts::PI).sqrt());
    let b = -0.5 * ((x - mean) / std_dev).powi(2);
    a * b.exp()
}

#[inline(always)]
pub fn fast_normal_approx(a: f64, x: f64) -> f64 {
    //This function provides a similar drop looking shape to the normal curve, but is much faster to calculate
    a / (((0.1 * a + 1.0) * a) + x * x)
}

pub trait RoundBy {
    fn round_by(self, n: Self) -> Self;
}
impl RoundBy for f32 {
    fn round_by(self, n: Self) -> Self {
        (self / n).round() * n
    }
}
impl RoundBy for f64 {
    fn round_by(self, n: Self) -> Self {
        (self / n).round() * n
    }
}
pub trait RoundEvenUp {
    fn round_even_up(self) -> Self;
}
impl RoundEvenUp for u32 {
    fn round_even_up(self) -> Self {
        match self {
            even if even % 2 == 0 => even + 1,
            odd => odd,
        }
    }
}
impl RoundEvenUp for u64 {
    fn round_even_up(self) -> Self {
        match self {
            even if even % 2 == 0 => even + 1,
            odd => odd,
        }
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

#[derive(Debug, Clone, Copy)]
pub struct VectorLine {
    start: Vec2,
    end: Vec2,
}
impl VectorLine {
    pub fn new(start: Vec2, end: Vec2) -> Self {
        Self { start, end }
    }
    pub fn get(&self, t: f32) -> Vec2 {
        self.start.lerp(self.end, t)
    }
    pub fn intersection(&self, rhs: &Self) -> Vec2 {
        //finds the intersection between two vector lines if it exists
        let a = self.start.x;
        let b = self.end.x;
        let c = rhs.start.x;
        let d = rhs.end.x;
        let e = self.start.y;
        let f = self.end.y;
        let g = rhs.start.y;
        let h = rhs.end.y;
        let s = (a * f - c * f + b * g - b * e) / (d * f - b * h);
        rhs.get(s)

        /* let s = (self.start.x * self.end.y + self.end.x * rhs.start.y
            - self.end.x * self.start.y
            - rhs.start.x * self.end.y)
            / (rhs.end.x * self.end.y - self.end.x * rhs.end.y);
        rhs.get(s) */
    }
    pub fn to_curve(&self) -> CubicCurve<Vec2> {
        straight_bezier_curve(self.start, self.end)
    }
}
