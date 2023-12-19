use std::{collections::HashMap, ops::Deref, ops::DerefMut};

use bevy::prelude::*;
use enum_map::EnumMap;
use itertools::Itertools;

use crate::{
    chunk::{
        chunk_tile_position::{CardinalDirection, TilePosition, WideTilePosition},
        DespawnEntityEvent,
    },
    math_utils::Mean,
    mesh_generator::create_road_intersection_mesh,
    world::heightmap::{HeightmapVertex, HeightmapsResource},
};

use super::{
    flatten_along_road, road_struct::Road, RoadTilesResource, SpawnRoadEvent, UpdateRoadMeshEvent,
};

#[derive(Event, Clone, Debug)]
pub struct SpawnIntersectionEvent {
    pub intersection: RoadIntersection,
}
impl Deref for SpawnIntersectionEvent {
    type Target = RoadIntersection;
    fn deref(&self) -> &Self::Target {
        &self.intersection
    }
}
impl DerefMut for SpawnIntersectionEvent {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.intersection
    }
}

pub fn spawn_intersection_meshes(
    mut commands: Commands,
    mut intersections: ResMut<RoadIntersectionsResource>,
    heightmaps: Res<HeightmapsResource>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    roads: Query<&Road>,
    mut gizmos: Gizmos,
) {
    for intersection in intersections.values_mut() {
        let connected_roads = intersection.roads.to_roads(&roads);
        //Find all the mesh intersection points
        /* let intersection_points = connected_roads
        .clone()
        .into_array()
        .into_iter()
        .flatten()
        .collect_vec()
        .into_iter()
        .circular_tuple_windows::<(_, _)>()
        .flat_map(|(a, b)| a.mesh_intersection(&b))
        .collect_vec(); */
        let intersection_points = connected_roads
            .clone()
            .into_array()
            .into_iter()
            .flatten()
            .tuple_combinations::<(_, _)>()
            .flat_map(|(a, b)| a.mesh_intersection(&b))
            .collect_vec();
        //Spawn a gizmo to highlight each of the mesh intersection points
        for intersection_point in intersection_points {
            let intersection_point =
                intersection_point.clamp(Vec2::new(0.0, 0.0), Vec2::new(64.0, 64.0));
            let mut pos = heightmaps.get_from_world_position_2d(intersection_point);
            pos.y += 0.1;
            gizmos.sphere(pos, Quat::IDENTITY, 0.1, Color::rgb(1.0, 0.0, 0.0));
        }
        if intersection.mesh.is_some() {
            continue;
        }
        //Retrieve the road objects which are connected to the intersection
        let mesh = create_road_intersection_mesh(intersection, &heightmaps, &connected_roads);
        let material = materials.add(Color::rgb(0.5, 0.5, 0.5).into());
        let entity = commands
            .spawn(PbrBundle {
                mesh: meshes.add(mesh),
                transform: Transform::from_translation(intersection.position.to_world_position()),
                material,
                ..Default::default()
            })
            .id();
        intersection.mesh = Some(entity);
    }
}

pub fn spawn_intersection_event_handler(
    mut events: EventReader<SpawnIntersectionEvent>,
    mut intersections: ResMut<RoadIntersectionsResource>,
    mut heightmaps: ResMut<HeightmapsResource>,
    mut occupied_tiles: ResMut<RoadTilesResource>,
    roads: Query<(Entity, &Road)>,
    mut spawn_roads_events: EventWriter<SpawnRoadEvent>,
    mut despawn_entity_events: EventWriter<DespawnEntityEvent>,
    mut update_road_mesh_events: EventWriter<UpdateRoadMeshEvent>,
) {
    for event in events.read() {
        //Check if the intersection is on a road section
        //Ignore the first and last tile since intersections are spawned there, otherwise we would get an infinite loop
        let mut removed_entities = Vec::new();
        for (entity, road) in roads.iter() {
            if road
                .center_line_tiles()
                .collect_vec()
                .into_iter()
                .dropping(1)
                .dropping_back(1)
                .contains(&event.position())
            {
                //Split the road into two sections
                let new_road_1 =
                    Road::new(road.starting_position(), event.position(), road.width());
                let new_road_2 = Road::new(event.position(), road.ending_position(), road.width());
                //Remove the old road
                despawn_entity_events.send(DespawnEntityEvent::new(entity));
                removed_entities.push(entity);
                //Spawn new roads
                spawn_roads_events.send(SpawnRoadEvent::new(new_road_1));
                spawn_roads_events.send(SpawnRoadEvent::new(new_road_2));
            }
        }
        //Replace the intersection if it already exists
        let intersection = if intersections.contains_key(&event.position()) {
            let mut new_intersection = intersections.get(&event.position()).unwrap().clone();
            for (direction, road) in &*event.roads {
                if let Some(road) = road {
                    new_intersection.roads[direction] = Some(*road);
                }
            }
            new_intersection.size = new_intersection.size.max(event.size);
            new_intersection.mesh = None;
            new_intersection
        } else {
            event.intersection.clone()
        };
        let tiles = event
            .tiles()
            .iter()
            .flat_map(|tile| tile.tile_neighbours().as_array().to_vec())
            .collect_vec();
        //Flatten the terrain including the neighbouring tiles
        let average_height = tiles
            .iter()
            .map(|tile| heightmaps[*tile])
            .mean_f32()
            .inner()
            .into_iter()
            .mean_f32();
        heightmaps.edit_tiles(
            tiles.as_slice(),
            &vec![HeightmapVertex::new([average_height; 4]); tiles.len()],
        );
        //Add the tiles to the occupied tiles
        for tile in event.tiles() {
            occupied_tiles.insert(*tile);
        }
        /* //Update the meshes of all connected roads to fix the terrain
        intersection.roads.iter().for_each(|(_, road)| {
            if let Some(road) = road {
                //Don't update the mesh if the road was split, since the current entity is already despawned
                if !removed_entities.contains(road) {
                    if let Ok((_, road)) = roads.get(*road) {
                        flatten_along_road(road, &mut heightmaps);
                    }
                    update_road_mesh_events.send(UpdateRoadMeshEvent::new(*road));
                }
            }
        }); */
        //Update the meshes of all roads to fix the terrain
        for (entity, road) in roads.iter() {
            flatten_along_road(road, &mut heightmaps);
            update_road_mesh_events.send(UpdateRoadMeshEvent::new(entity));
        }
        intersections.insert(event.position(), intersection);
    }
}
pub fn remove_redundant_intersections(
    events: EventReader<SpawnIntersectionEvent>,
    mut intersections: ResMut<RoadIntersectionsResource>,
    mut spawn_roads_events: EventWriter<SpawnRoadEvent>,
    mut despawn_entity_events: EventWriter<DespawnEntityEvent>,
    roads: Query<&Road>,
) {
    if events.is_empty() {
        return;
    }
    let mut to_remove = Vec::new();
    for (_, intersection) in intersections.iter() {
        //Check if intersection is redundant
        let intersection_roads = intersection
            .roads
            .into_iter()
            .filter_map(|(_, road_option)| {
                road_option.and_then(|road_entity| match roads.get(road_entity) {
                    Ok(road) => Some((road_entity, road)),
                    Err(_) => None,
                })
            })
            .collect_vec();
        if intersection_roads.len() == 2
            && intersection_roads[0].1.direction() == intersection_roads[1].1.direction()
            && intersection_roads[0].1.width() == intersection_roads[1].1.width()
        {
            //Add the intersection to the list of intersections to remove
            to_remove.push(intersection.position());
            //Join the two roads
            let new_road = Road::new(
                intersection_roads[0].1.starting_position(),
                intersection_roads[1].1.ending_position(),
                intersection_roads[0].1.width(),
            );
            //Spawn the new road
            spawn_roads_events.send(SpawnRoadEvent::new(new_road));
            //Remove the old roads
            despawn_entity_events.send(DespawnEntityEvent::new(intersection_roads[0].0));
            despawn_entity_events.send(DespawnEntityEvent::new(intersection_roads[1].0));
        }
    }
    //Remove the redundant intersections
    for position in to_remove {
        intersections.remove(&position);
    }
}

#[derive(Resource, Default, Debug, Clone, Eq, PartialEq)]
pub struct RoadIntersectionsResource(HashMap<TilePosition, RoadIntersection>);
impl RoadIntersectionsResource {
    pub fn contains_wide_tile(&self, tile: WideTilePosition) -> bool {
        tile.tiles().any(|tile| self.contains_key(&tile))
    }
}
impl Deref for RoadIntersectionsResource {
    type Target = HashMap<TilePosition, RoadIntersection>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for RoadIntersectionsResource {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct RoadIntersection {
    position: TilePosition,
    pub size: u32,
    pub roads: ConnectedRoads,
    tiles: Vec<TilePosition>,
    mesh: Option<Entity>,
}
impl RoadIntersection {
    pub fn new(position: TilePosition, size: u32, roads: ConnectedRoads) -> Self {
        Self {
            position,
            size,
            roads,
            tiles: Self::calculate_tiles(position, size),
            mesh: None,
        }
    }
    pub fn position(&self) -> TilePosition {
        self.position
    }
    pub fn tiles(&self) -> &[TilePosition] {
        &self.tiles
    }
    fn calculate_tiles(starting_position: TilePosition, size: u32) -> Vec<TilePosition> {
        WideTilePosition::new(starting_position, size)
            .tiles()
            .collect_vec()
        /* starting_position.tiles_from_size(size).collect_vec() */
    }
    pub fn connected_road_vectors<'a>(
        &'a self,
        heightmaps: &'a HeightmapsResource,
    ) -> impl Iterator<Item = (Vec3, Vec3)> + '_ {
        self.roads
            .iter()
            .filter_map(move |(direction, road)| match road {
                Some(_road) => {
                    let mut starting_position =
                        heightmaps.get_from_world_position(self.position.to_world_position());
                    starting_position.y += 0.2;
                    let mut ending_position = heightmaps
                        .get_from_world_position((self.position + direction).to_world_position());
                    ending_position.y += 0.2;
                    Some((starting_position, ending_position))
                }
                None => None,
            })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Default)]
pub struct ConnectedRoads(EnumMap<CardinalDirection, Option<Entity>>);
impl ConnectedRoads {
    pub fn tiles(&self, roads: &Query<&Road>) -> Vec<TilePosition> {
        self.iter()
            .filter_map(move |(_, road)| {
                road.as_ref()
                    .map(|road| roads.get(*road).unwrap().tiles().clone())
            })
            .flatten()
            .map(|(a, _)| a)
            .collect_vec()
    }
    pub fn to_roads(&self, roads: &Query<&Road>) -> EnumMap<CardinalDirection, Option<Road>> {
        let mut connected_roads = EnumMap::default();
        for (direction, road) in self.iter() {
            if let Some(road) = road {
                if let Ok(road) = roads.get(*road) {
                    connected_roads[direction] = Some(road.clone());
                }
            }
        }
        connected_roads
    }
}
impl Deref for ConnectedRoads {
    type Target = EnumMap<CardinalDirection, Option<Entity>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for ConnectedRoads {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
/* pub struct SlopeInterceptLine {
    slope: f32,
    intercept: f32,
}
impl SlopeInterceptLine {
    pub fn new(starting: Vec2, ending: Vec2) -> Self {
        let slope = (ending.y - starting.y) / (ending.x - starting.x);
        let intercept = starting.y - slope * starting.x;
        Self { slope, intercept }
    }
    pub fn intersection(self, rhs: SlopeInterceptLine) -> Vec2 {}
}
 */
