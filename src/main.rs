use bevy::prelude::*;
use bevy::log;
use board_plugin::{BoardPlugin, resources::BoardOptions};

#[cfg(feature = "debug")]
use bevy_inspector_egui::WorldInspectorPlugin;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AppState {
    InGame,
    Out,
}

fn main() {
    let mut app = App::new();

    app.insert_resource(WindowDescriptor {
        title: "mine sweeper".to_string(),
        width: 700.0,
        height: 800.0,
        ..Default::default()
    })
    .add_state(AppState::InGame)
    .add_plugins(DefaultPlugins)
    .add_plugin(BoardPlugin {
        running_state: AppState::InGame,
    })
    .add_system(state_handler);

    app.insert_resource(BoardOptions {
        map_size: (20, 20),
        bomb_count: 40,
        tile_padding: 3.0,
        safe_start: true,
        ..Default::default()
    });

    #[cfg(feature = "debug")]
    app.add_plugin(WorldInspectorPlugin::new());

    app.add_startup_system(camera_setup);
    app.run();
}

fn state_handler(mut state: ResMut<State<AppState>>, keys: Res<Input<KeyCode>>) {
    if keys.just_pressed(KeyCode::C) {
        log::debug!("clearing detected");
        if state.current() == &AppState::InGame {
            log::info!("clearing game");
            state.set(AppState::Out).unwrap();
        }
    }
    if keys.just_pressed(KeyCode::G) {
        log::debug!("loading detected");
        if state.current() == &AppState::Out {
            log::info!("loading game");
            state.set(AppState::InGame).unwrap();
        } else if state.current() == &AppState::InGame {
            log::info!("loading game");
            state.restart().unwrap();
        }
    }
    if keys.just_pressed(KeyCode::Escape) {
        log::debug!("{:?}", state.inactives());
        if state.current() == &AppState::InGame {
            state.overwrite_push(AppState::Out).unwrap();
        } else if !state.inactives().is_empty() {
            state.pop().unwrap();
        }
    }
}

fn camera_setup(mut commands: Commands) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
}