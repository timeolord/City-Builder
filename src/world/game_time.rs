use bevy::{ecs::schedule::ScheduleLabel, prelude::*};

use crate::GameState;

pub struct GameTimePlugin;

impl Plugin for GameTimePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::World), setup);
        app.add_systems(PostUpdate, run_game_update.run_if(every_other_time().and_then(in_state(GameState::World))));
        app.add_systems(Update, (input).run_if(in_state(GameState::World)));
        app.add_systems(OnExit(GameState::World), exit);
    }
}

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct GameUpdate;

#[derive(Resource)]
pub struct GameTime {
    pub relative_time: usize,
}

fn every_other_time() -> impl Condition<()> {
    IntoSystem::into_system(|mut flag: Local<bool>| {
        *flag = !*flag;
        *flag
    })
}


fn setup(mut commands: Commands) {
    commands.insert_resource(GameTime { relative_time: 1 });
}

fn exit(mut commands: Commands) {
    commands.remove_resource::<GameTime>();
}

fn run_game_update(world: &mut World) {
    let _ = world.try_schedule_scope(GameUpdate, |world, schedule| {
        let relative_time = world.resource::<GameTime>().relative_time;
        for _ in 0..relative_time {
            schedule.run(world);
        }
    });
}

fn input(
    keyboard: Res<Input<KeyCode>>,
    mut game_time_res: ResMut<GameTime>,
    mut previous_time: Local<usize>,
) {
    if keyboard.just_pressed(KeyCode::P) {
        if game_time_res.relative_time == 0 {
            game_time_res.relative_time = *previous_time;
        } else {
            *previous_time = game_time_res.relative_time;
            game_time_res.relative_time = 0;
        }
    }
    if keyboard.just_pressed(KeyCode::Q) {
        game_time_res.relative_time *= 2;
    }
    if keyboard.just_pressed(KeyCode::E) {
        game_time_res.relative_time /= 2;
    }
    if game_time_res.relative_time < 1 {
        game_time_res.relative_time = 1;
    }
}
