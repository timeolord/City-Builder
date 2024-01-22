use bevy::prelude::*;

use crate::GameState;

pub struct MainMenuPlugin;

impl Plugin for MainMenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MenuEntitiesResource>();
        app.add_systems(OnEnter(GameState::MainMenu), setup);
        app.add_systems(Update, input.run_if(in_state(GameState::MainMenu)));
        app.add_systems(OnExit(GameState::MainMenu), exit);
    }
}

#[derive(Component)]
struct NewGameButton;

#[derive(Resource, Default)]
struct MenuEntitiesResource {
    entities: Vec<Entity>,
}

const NORMAL_BUTTON: Color = Color::rgb(0.15, 0.15, 0.15);

fn exit(mut commands: Commands, mut menu_entities: ResMut<MenuEntitiesResource>) {
    let menu_entities_vec = &mut menu_entities.as_mut().entities;
    for entity in menu_entities_vec.drain(..) {
        commands.entity(entity).despawn_recursive();
    }
    menu_entities_vec.clear();
}

fn setup(mut commands: Commands, mut menu_entities: ResMut<MenuEntitiesResource>) {
    let menu_entities_vec = &mut menu_entities.as_mut().entities;

    menu_entities_vec.push(commands.spawn(Camera2dBundle::default()).id());

    let background_node = NodeBundle {
        style: Style {
            width: Val::Percent(100.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        ..default()
    };
    let mut background_node = commands.spawn(background_node);
    menu_entities_vec.push(background_node.id());

    background_node.with_children(create_new_game_button("New Game".to_owned()));
}

fn create_new_game_button(button_text: String) -> impl Fn(&mut ChildBuilder<'_, '_, '_>) {
    move |parent: &mut ChildBuilder<'_, '_, '_>| {
        parent
            .spawn(ButtonBundle {
                style: Style {
                    width: Val::Px(150.0),
                    height: Val::Px(65.0),
                    border: UiRect::all(Val::Px(5.0)),
                    // horizontally center child text
                    justify_content: JustifyContent::Center,
                    // vertically center child text
                    align_items: AlignItems::Center,
                    ..default()
                },
                border_color: BorderColor(Color::BLACK),
                background_color: NORMAL_BUTTON.into(),
                ..default()
            })
            .insert(NewGameButton)
            .with_children(|parent| {
                parent.spawn(TextBundle::from_section(
                    button_text.clone(),
                    TextStyle {
                        font_size: 20.0,
                        color: Color::rgb(0.9, 0.9, 0.9),
                        ..default()
                    },
                ));
            });
    }
}

fn input(
    mut start_game_query: Query<&Interaction, (Changed<Interaction>, With<NewGameButton>)>,
    mut game_state: ResMut<NextState<GameState>>,
) {
    for start_game in &mut start_game_query {
        match *start_game {
            Interaction::Pressed => game_state.set(GameState::World),
            Interaction::Hovered | Interaction::None => {}
        }
    }
}
