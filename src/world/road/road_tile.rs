use crate::chunk::chunk_tile_position::{CardinalDirection, TilePosition};

#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoadTile {
    pub position: TilePosition,
    pub direction: CardinalDirection,
    pub diagonal: Option<CardinalDirection>,
}
impl Default for RoadTile {
    fn default() -> Self {
        Self {
            position: TilePosition::default(),
            direction: CardinalDirection::North,
            diagonal: None,
        }
    }
}
