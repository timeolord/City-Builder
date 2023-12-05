use bevy::prelude::*;

use crate::{
    chunk::chunk_tile_position::{TilePosition, TilePosition2D},
    constants::DEBUG,
    world::{buildings::NeedsPathFinding, tile_highlight::{HighlightTileEvent, Duration}},
    GameState,
};

use super::RoadTilesResource;

pub struct PathfindingPlugin;

impl Plugin for PathfindingPlugin {
    fn build(&self, app: &mut App) {
        //app.add_systems(OnEnter(GameState::World), setup);
        app.add_systems(
            Update,
            (find_path_event_handler).run_if(in_state(GameState::World)),
        );
        //app.add_systems(
        //    Update,
        //    (highlight_road_intersections).run_if(in_state(GameState::World)),
        //);
        //app.add_systems(OnExit(GameState::World), exit);
    }
}

#[derive(Component)]
pub struct Pathfind {
    pub path: Path,
    pub current_index: usize,
}
pub type Distance = usize;
pub type Path = Vec<TilePosition2D>;


fn find_path_event_handler(
    mut commands: Commands,
    mut pathfind_query: Query<(Entity, &NeedsPathFinding)>,
    occupied_road_tiles: Res<RoadTilesResource>,
    mut highlight_tile_events: EventWriter<HighlightTileEvent>,
) {
    for (entity, pathfind) in &mut pathfind_query {
        let start = pathfind.start.position_2d();
        let end = pathfind.end.position_2d();
        let path: Option<(Path, Distance)> = pathfinding::prelude::dijkstra(
            &start,
            |p| {
                occupied_road_tiles
                    .get_neighbours(TilePosition::from_position_2d(*p))
                    .map(|p| (p.position_2d(), 1))
            },
            |p| *p == end,
        );

        match path {
            Some((path, _distance)) => {
                for position in &path {
                    highlight_tile_events.send(HighlightTileEvent {
                        position: TilePosition::from_position_2d(*position),
                        color: Color::GOLD,
                        duration: Duration::Timed(std::time::Duration::from_secs(1)),
                    });
                }

                commands
                    .entity(entity)
                    .remove::<NeedsPathFinding>()
                    .insert(Pathfind {
                        path,
                        current_index: 0,
                    });
            }
            None => {
                if DEBUG {
                    println!("No path found between {start:?} and {end:?}");
                }
            }
        }
    }
}
