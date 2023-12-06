use crate::chunk::chunk_tile_position::{TilePosition};

#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Default)]
pub struct RoadTile {
    pub position: TilePosition,
    //pub direction: CardinalDirection,
}

