use bevy::prelude::*;

use crate::{
    chunk::chunk_tile_position::{ChunkTilePosition, TilePosition2D},
    constants::DEBUG,
    world::{buildings::NeedsPathFinding, tile_highlight::HighlightTileEvent},
    GameState,
};

use super::OccupiedRoadTiles;

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

/* fn test(
    mut find_path_events: EventWriter<FindPathEvent>,
    occupied_road_tiles: Res<OccupiedRoadTiles>,
    keyboard: Res<Input<KeyCode>>,
) {
    if keyboard.just_pressed(KeyCode::H) {
        let random_road_tile_1_index = rand::random::<usize>() % occupied_road_tiles.tiles.len();
        let random_road_tile_2_index = rand::random::<usize>() % occupied_road_tiles.tiles.len();
        let random_road_tile_1 = occupied_road_tiles
            .tiles
            .keys()
            .nth(random_road_tile_1_index)
            .unwrap();
        let random_road_tile_2 = occupied_road_tiles
            .tiles
            .keys()
            .nth(random_road_tile_2_index)
            .unwrap();
        find_path_events.send(FindPathEvent {
            start: *random_road_tile_1,
            end: *random_road_tile_2,
        });
    }
} */

fn find_path_event_handler(
    mut commands: Commands,
    mut pathfind_query: Query<(Entity, &NeedsPathFinding)>,
    occupied_road_tiles: Res<OccupiedRoadTiles>,
    mut highlight_tile_events: EventWriter<HighlightTileEvent>,
) {
    for (entity, pathfind) in pathfind_query.iter_mut() {
        let start: [usize; 2] = pathfind.start.as_tile_position_2d();
        let end = pathfind.end.as_tile_position_2d();
        let path: Option<(Path, Distance)> = pathfinding::prelude::dijkstra(
            &start,
            |p| {
                occupied_road_tiles
                    .get_neighbours(ChunkTilePosition::from_tile_position_2d(*p))
                    .into_iter()
                    .map(|p| (p.as_tile_position_2d(), 1))
            },
            |p| *p == end,
        );

        match path {
            Some((path, _distance)) => {
                for position in path.iter() {
                    highlight_tile_events.send(HighlightTileEvent {
                        position: ChunkTilePosition::from_tile_position_2d(*position),
                        color: Color::GOLD,
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
                    println!("No path found between {:?} and {:?}", start, end);
                }
            }
        }
    }
}
