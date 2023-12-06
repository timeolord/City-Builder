use itertools::Itertools;
use std::{collections::HashSet, f32::consts::PI};

use bevy::{math::cubic_splines::CubicCurve, prelude::*};

use crate::{
    chunk::chunk_tile_position::{CardinalDirection, TilePosition},
    constants::TILE_SIZE,
    math_utils::{straight_bezier_curve, Arclength, RoundBy},
    world::heightmap::HeightmapsResource,
};

use super::road_tile::RoadTile;

#[derive(Component, Clone, Debug)]
pub struct Road {
    starting_position: TilePosition,
    ending_position: TilePosition,
    width: u32,
    bezier_curve: CubicCurve<Vec2>,
    length: f32,
    tiles: Vec<(TilePosition, RoadTile)>,
    direction: CardinalDirection,
}
impl Road {
    pub fn new(starting_position: TilePosition, ending_position: TilePosition, width: u32) -> Self {
        let bezier_curve = straight_bezier_curve(
            starting_position.to_world_position_2d(),
            ending_position.to_world_position_2d(),
        );
        let length = bezier_curve.arclength();
        let mut result = Self {
            starting_position,
            ending_position,
            width,
            bezier_curve,
            length,
            tiles: Vec::new(),
            direction: Self::calculate_direction(starting_position, ending_position),
        };
        result.calculate_road_tiles();
        result
    }
    fn calculate_direction(
        starting_position: TilePosition,
        ending_position: TilePosition,
    ) -> CardinalDirection {
        let starting_vec = starting_position.position_2d();
        let current_vec = ending_position.position_2d();
        let relative_vec = current_vec - starting_vec;
        let angle = (relative_vec.y as f32).atan2(relative_vec.x as f32) * 180.0 / PI;
        match angle as i32 {
            0 => CardinalDirection::North,
            45 => CardinalDirection::NorthEast,
            90 => CardinalDirection::East,
            135 => CardinalDirection::SouthEast,
            180 => CardinalDirection::South,
            -45 => CardinalDirection::NorthWest,
            -90 => CardinalDirection::West,
            -135 => CardinalDirection::SouthWest,
            -180 => CardinalDirection::South,
            _ => {
                panic!("Unexpected angle: {angle}");
            }
        }
    }
    pub fn starting_position(&self) -> TilePosition {
        self.starting_position
    }
    pub fn ending_position(&self) -> TilePosition {
        self.ending_position
    }
    pub fn width(&self) -> u32 {
        self.width
    }
    pub fn direction(&self) -> CardinalDirection {
        self.direction
    }
    pub fn length(&self) -> f32 {
        self.length
    }
    pub fn subdivisions(&self) -> usize {
        let road_length = self.length().round() as usize;
        let subdivisions = road_length * TILE_SIZE as usize;
        subdivisions * 2
    }
    pub fn tiles(&self) -> &Vec<(TilePosition, RoadTile)> {
        &self.tiles
    }
    pub fn normal_vectors(&self) -> impl Iterator<Item = Vec2> + '_ {
        self.normal_vectors_with_subdivisions(self.subdivisions())
    }
    pub fn normal_vectors_with_subdivisions(
        &self,
        subdivision: usize,
    ) -> impl Iterator<Item = Vec2> + '_ {
        self.bezier_curve.iter_velocities(subdivision).map(|v| {
            //Rotate velocity vector 90 degrees
            let rotated = Vec2::new(v.y, -v.x);
            //Normalize vector
            rotated.normalize_or_zero()
        })
    }
    pub fn as_world_positions<'a>(
        &'a self,
        heightmaps: &'a HeightmapsResource,
        height_offset: f32,
        horizontal_offset: f32,
    ) -> impl Iterator<Item = Vec3> + '_ {
        self.as_2d_positions(horizontal_offset).map(move |p| {
            let mut position = heightmaps.get_from_world_position_2d(p);
            position.y += height_offset;
            position
        })
    }
    pub fn as_2d_positions(&self, horizontal_offset: f32) -> impl Iterator<Item = Vec2> + '_ {
        self.as_2d_positions_with_subdivision(horizontal_offset, self.subdivisions())
    }
    pub fn as_2d_positions_with_subdivision(
        &self,
        horizontal_offset: f32,
        subdivision: usize,
    ) -> impl Iterator<Item = Vec2> + '_ {
        self.bezier_curve
            .iter_positions(subdivision)
            .zip_eq(self.normal_vectors_with_subdivisions(subdivision))
            //We round here to prevent floating point errors from screwing us over later. Like 0.9999999999999999 instead of 1.0
            .map(move |(p, normal)| {
                Vec2::new(p.x.round_by(0.1), p.y.round_by(0.1)) + (normal * horizontal_offset)
            })
    }
    fn calculate_road_tiles(&mut self) {
        let subdivison_multipler = 10;
        self.tiles =
            self.calculate_road_tiles_with_subdivisions(self.subdivisions() * subdivison_multipler);
    }
    fn calculate_road_tiles_with_subdivisions(
        &mut self,
        subdivisions: usize,
    ) -> Vec<(TilePosition, RoadTile)> {
        let mut road_tiles: Vec<(TilePosition, RoadTile)> = Vec::new();
        let road_width = (self.width as f32 / 2.0) - (self.width as f32 / 1000.0);

        let positions = self
            .as_2d_positions_with_subdivision(-road_width, subdivisions)
            .zip_eq(self.as_2d_positions_with_subdivision(road_width, subdivisions));
        for (starting, ending) in positions {
            let curve = straight_bezier_curve(starting, ending);
            let curve_length = curve.arclength().ceil() as usize;
            let subdivisions = curve_length * TILE_SIZE as usize;
            let curve_positions = curve.iter_positions(subdivisions);
            for position in curve_positions {
                let position = Vec3::new(position.x, 0.0, position.y);
                let position = TilePosition::from_world_position(position);
                road_tiles.push((
                    position,
                    RoadTile {
                        position,
                        //direction: self.direction(),
                    },
                ));
            }
        }
        road_tiles.into_iter().unique().collect_vec()
    }

    pub fn row_tiles(&self) -> Vec<Vec<(TilePosition, RoadTile)>> {
        let mut road_tiles = Vec::new();
        let road_width = (self.width as f32 / 2.0) - (self.width as f32 / 1000.0);
        let subdivisions = self.tile_subdivision();

        let positions = self
            .as_2d_positions_with_subdivision(-road_width, subdivisions)
            .zip_eq(self.as_2d_positions_with_subdivision(road_width, subdivisions));
        for (starting, ending) in positions {
            let curve = straight_bezier_curve(starting, ending);
            let curve_length = curve.arclength().ceil() as usize;
            let subdivisions = curve_length * TILE_SIZE as usize;
            let curve_positions = curve.iter_positions(subdivisions);
            let mut row_tiles = Vec::new();
            for position in curve_positions {
                let position = Vec3::new(position.x, 0.0, position.y);
                let position = TilePosition::from_world_position(position);
                row_tiles.push((
                    position,
                    RoadTile {
                        position,
                        //direction: self.direction(),
                    },
                ));
            }
            road_tiles.push(row_tiles);
        }
        road_tiles.into_iter().unique().collect_vec()
    }
    fn tile_subdivision(&self) -> usize {
        match self.direction {
            CardinalDirection::North
            | CardinalDirection::South
            | CardinalDirection::East
            | CardinalDirection::West => self.length().round() as usize,
            //Subdivide the diagonal roads by the length of the hypotenuse so that each segment is exactly one tile
            CardinalDirection::NorthEast
            | CardinalDirection::SouthWest
            | CardinalDirection::NorthWest
            | CardinalDirection::SouthEast => (self.length() / 2.0f32.sqrt()).round() as usize,
        }
    }
    pub fn center_line_tiles(&self) -> impl Iterator<Item = TilePosition> + '_ {
        self.as_2d_positions_with_subdivision(0.0, self.tile_subdivision())
            .map(|p| TilePosition::from_world_position(Vec3::new(p.x, 0.0, p.y)))
    }
    pub fn intersection(&self, rhs: &Self) -> Option<TilePosition> {
        let self_center_tiles = self.center_line_tiles().collect::<HashSet<_>>();
        let rhs_center_tiles = rhs.center_line_tiles().collect::<HashSet<_>>();
        let intersection = self_center_tiles.intersection(&rhs_center_tiles);
        intersection.copied().next()
    }
    /* fn slope_intercept_line(&self) -> SlopeInterceptLine {
        SlopeInterceptLine::new(
            self.starting_position.to_world_position_2d(),
            self.ending_position.to_world_position_2d(),
        )
    }
    pub fn intersection(&self, rhs: &Road) -> Option<Vec2> {
        let max_x = self
            .starting_position
            .to_world_position_2d()
            .x
            .max(self.ending_position.to_world_position_2d().x);
        let min_x = self
            .starting_position
            .to_world_position_2d()
            .x
            .min(self.ending_position.to_world_position_2d().x);
        let max_y = self
            .starting_position
            .to_world_position_2d()
            .y
            .max(self.ending_position.to_world_position_2d().y);
        let min_y = self
            .starting_position
            .to_world_position_2d()
            .y
            .min(self.ending_position.to_world_position_2d().y);
        let lhs_line = self.slope_intercept_line();
        let rhs_line = rhs.slope_intercept_line();
        let point = lhs_line.intersection(rhs_line);
        if point.x >= min_x && point.x <= max_x && point.y >= min_y && point.y <= max_y {
            Some(point)
        } else {
            None
        }
    } */
}
