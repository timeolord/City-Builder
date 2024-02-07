use std::ops::{Add, Neg};

use enum_map::Enum;
use strum_macros::EnumIter;

#[derive(Enum, EnumIter, Hash, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CardinalDirection {
    North = 0,
    NorthEast = 1,
    East = 2,
    SouthEast = 3,
    South = 4,
    SouthWest = 5,
    West = 6,
    NorthWest = 7,
}

impl CardinalDirection {
    pub fn non_compound_directions() -> impl Iterator<Item = CardinalDirection> {
        [
            CardinalDirection::North,
            CardinalDirection::East,
            CardinalDirection::South,
            CardinalDirection::West,
        ]
        .into_iter()
    }
    pub fn split_direction(self) -> [CardinalDirection; 2] {
        match self {
            CardinalDirection::North => {
                [CardinalDirection::NorthWest, CardinalDirection::NorthEast]
            }
            CardinalDirection::NorthEast => [CardinalDirection::North, CardinalDirection::East],
            CardinalDirection::East => [CardinalDirection::NorthEast, CardinalDirection::SouthEast],
            CardinalDirection::SouthEast => [CardinalDirection::East, CardinalDirection::South],
            CardinalDirection::South => {
                [CardinalDirection::SouthEast, CardinalDirection::SouthWest]
            }
            CardinalDirection::SouthWest => [CardinalDirection::South, CardinalDirection::West],
            CardinalDirection::West => [CardinalDirection::SouthWest, CardinalDirection::NorthWest],
            CardinalDirection::NorthWest => [CardinalDirection::West, CardinalDirection::North],
        }
    }
    pub fn next_clockwise(self) -> CardinalDirection {
        match self {
            CardinalDirection::North => CardinalDirection::NorthEast,
            CardinalDirection::NorthEast => CardinalDirection::East,
            CardinalDirection::East => CardinalDirection::SouthEast,
            CardinalDirection::SouthEast => CardinalDirection::South,
            CardinalDirection::South => CardinalDirection::SouthWest,
            CardinalDirection::SouthWest => CardinalDirection::West,
            CardinalDirection::West => CardinalDirection::NorthWest,
            CardinalDirection::NorthWest => CardinalDirection::North,
        }
    }
    pub fn next_counter_clockwise(self) -> CardinalDirection {
        match self {
            CardinalDirection::North => CardinalDirection::NorthWest,
            CardinalDirection::NorthEast => CardinalDirection::North,
            CardinalDirection::East => CardinalDirection::NorthEast,
            CardinalDirection::SouthEast => CardinalDirection::East,
            CardinalDirection::South => CardinalDirection::SouthEast,
            CardinalDirection::SouthWest => CardinalDirection::South,
            CardinalDirection::West => CardinalDirection::SouthWest,
            CardinalDirection::NorthWest => CardinalDirection::West,
        }
    }
    pub fn to_angle(self) -> f32 {
        match self {
            CardinalDirection::North => 0.0,
            CardinalDirection::NorthEast => 45.0,
            CardinalDirection::East => 90.0,
            CardinalDirection::SouthEast => 135.0,
            CardinalDirection::South => 180.0,
            CardinalDirection::SouthWest => -135.0,
            CardinalDirection::West => -90.0,
            CardinalDirection::NorthWest => -45.0,
        }
    }
    pub fn all_left_of(self) -> Vec<CardinalDirection> {
        let mut directions = Vec::new();
        let mut current_direction = self;
        for _ in 0..3 {
            current_direction = current_direction.next_counter_clockwise();
            directions.push(current_direction);
        }
        directions
    }
    pub fn all_right_of(self) -> Vec<CardinalDirection> {
        let mut directions = Vec::new();
        let mut current_direction = self;
        for _ in 0..3 {
            current_direction = current_direction.next_clockwise();
            directions.push(current_direction);
        }
        directions
    }
}
impl Neg for CardinalDirection {
    type Output = Self;

    fn neg(self) -> Self::Output {
        match self {
            CardinalDirection::North => CardinalDirection::South,
            CardinalDirection::NorthEast => CardinalDirection::SouthWest,
            CardinalDirection::East => CardinalDirection::West,
            CardinalDirection::SouthEast => CardinalDirection::NorthWest,
            CardinalDirection::South => CardinalDirection::North,
            CardinalDirection::SouthWest => CardinalDirection::NorthEast,
            CardinalDirection::West => CardinalDirection::East,
            CardinalDirection::NorthWest => CardinalDirection::SouthEast,
        }
    }
}
impl Add<CardinalDirection> for [i32; 2] {
    type Output = [i32; 2];

    fn add(self, rhs: CardinalDirection) -> Self::Output {
        match rhs {
            CardinalDirection::North => [self[0], self[1] + 1],
            CardinalDirection::NorthEast => [self[0] + 1, self[1] + 1],
            CardinalDirection::East => [self[0] + 1, self[1]],
            CardinalDirection::SouthEast => [self[0] + 1, self[1] - 1],
            CardinalDirection::South => [self[0], self[1] - 1],
            CardinalDirection::SouthWest => [self[0] - 1, self[1] - 1],
            CardinalDirection::West => [self[0] - 1, self[1]],
            CardinalDirection::NorthWest => [self[0] - 1, self[1] + 1],
        }
    }
}