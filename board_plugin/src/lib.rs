pub mod components;
pub mod resources;
pub mod systems;
mod bounds;
mod events;

use bevy::ecs::schedule::StateData;
#[cfg(feature = "debug")]
use bevy_inspector_egui::InspectableRegistry;

use bevy::{log, prelude::*, math::Vec3Swizzles, utils::HashMap};
use resources::{tile_map::TileMap, BoardOptions, tile::Tile, TileSize, BoardPosition, Board, BoardAssets};
use components::{Coordinates, BombNeighbor, Bomb, Uncover};
use crate::events::*;

use crate::bounds::Bounds2;

pub struct BoardPlugin<T> {
    pub running_state: T,
}

impl <T: StateData>Plugin for BoardPlugin<T> {
    fn build(&self, app: &mut App) {
        // app.add_startup_system(Self::create_board)
        //     .add_system(systems::input::input_handling)
        //     .add_system(systems::uncover::trigger_event_handler)
        //     .add_system(systems::uncover::uncover_tiles)
        //     .add_event::<TileTriggerEvent>();
        
        // When the running states comes into the stack we load a board
        app.add_system_set(
            SystemSet::on_enter(self.running_state.clone())
                .with_system(Self::create_board)
        )
        .add_system_set(
            SystemSet::on_pause(self.running_state.clone())
        )
        // We handle input and trigger events only if the state is active
        .add_system_set(
            SystemSet::on_update(self.running_state.clone())
                .with_system(systems::input::input_handling)
                .with_system(systems::uncover::trigger_event_handler),
        )
        // We handle uncovering even if the state is inactive
        .add_system_set(
            SystemSet::on_in_stack_update(self.running_state.clone())
                .with_system(systems::uncover::uncover_tiles),   
        )
        .add_system_set(
            SystemSet::on_exit(self.running_state.clone())
                .with_system(Self::cleanup_board)
        )
        .add_event::<TileTriggerEvent>();

        #[cfg(feature = "debug")]
        {
            let mut registry = app
                .world
                .get_resource_or_insert_with(InspectableRegistry::default);

            registry.register::<Coordinates>();
            registry.register::<BombNeighbor>();
            registry.register::<Bomb>();
            // app.register_inspectable::<Uncover>();
        }
        log::info!("Loaded Board Plugin");
    }
}

impl<T> BoardPlugin<T> {
    pub fn create_board(
        mut commands: Commands,
        board_options: Option<Res<BoardOptions>>,
        board_assets: Res<BoardAssets>,
        window: Res<WindowDescriptor>,
    ) {
        let options = match board_options {
            None => BoardOptions::default(),
            Some(o) => o.clone(),
        };
        let mut tile_map = TileMap::empty(options.map_size.0, options.map_size.1);
        let tile_size = match options.tile_size {
            TileSize::Fixed(v) => v,
            TileSize::Adaptative { min, max } => Self::adaptative_tile_size(
                window, 
                (min, max),
                (tile_map.width(), tile_map.height()),
            ),
        };
        let board_size = Vec2::new(
            tile_map.width() as f32 * tile_size,
            tile_map.height() as f32 * tile_size,
        );
        let board_position = match options.position {
            BoardPosition::Centered { offset } => { Vec3::new(
                -(board_size.x / 2.0) ,
                -(board_size.y / 2.0),
                0.0) + offset
            }
            BoardPosition::Custom(p) => p,
        };
        
        let mut covered_tiles = HashMap::with_capacity((tile_map.width() * tile_map.height()).into());
        let mut safe_start = None;

        log::info!("Board size: {}", board_size);
        tile_map.set_bombs(40);
        
        let board_entity = commands.spawn()
            .insert(Name::new("Board"))
            .insert(Transform::from_translation(board_position))
            .insert(GlobalTransform::default())
            .with_children(|parent| {
                parent
                    .spawn_bundle(SpriteBundle {
                        sprite: Sprite {
                            color: board_assets.board_material.color,
                            custom_size: Some(board_size),
                            ..Default::default()
                        },
                        texture: board_assets.board_material.texture.clone(),
                        transform: Transform::from_xyz(board_size.x / 2.0, board_size.y / 2.0, 0.0),
                        ..Default::default()
                    })
                    .insert(Name::new("Background"));
                Self::spawn_tiles(
                    parent,
                    &tile_map,
                    tile_size,
                    options.tile_padding,
                    &board_assets,
                    &mut covered_tiles,
                    &mut safe_start,
                )
            })
            .id();
        #[cfg(feature = "debug")]
        log::info!("{}", tile_map.console_output());

        if options.safe_start {
            if let Some(entity) = safe_start {
                commands.entity(entity).insert(Uncover);
            }
        }

        commands.insert_resource(Board {
            tile_map,
            bounds: Bounds2 {
                position: board_position.xy(),
                size: board_size,
            },
            tile_size,
            covered_tiles,
            entity: board_entity
        });
    }

    fn bomb_count_text_bundle(count: u8, board_assets: &BoardAssets, size: f32) -> Text2dBundle {
        let color = board_assets.bomb_counter_color(count);

        Text2dBundle {
            text: Text {
                sections: vec![TextSection {
                    value: count.to_string(),
                    style: TextStyle {
                        font: board_assets.bomb_counter_font.clone(),
                        font_size: size,
                        color
                    }
                }],
                alignment: TextAlignment {
                    vertical: VerticalAlign::Center,
                    horizontal: HorizontalAlign::Center
                } 
            },
            transform: Transform::from_xyz(0.0, 0.0, 1.0),
            ..Default::default()
        }
    }

    fn spawn_tiles(
        parent: &mut ChildBuilder,
        tile_map: &TileMap,
        size: f32,
        padding: f32,
        board_assets: &BoardAssets,
        covered_tiles: &mut HashMap<Coordinates, Entity>,
        safe_start_entity: &mut Option<Entity>,
    ) {
        for (y, line) in tile_map.iter().enumerate() {
            for (x, tile) in line.iter().enumerate() {
                let mut cmd = parent.spawn();
                let coordinates = Coordinates {
                    x: x as u16,
                    y: y as u16,
                };

                cmd.insert_bundle(SpriteBundle {
                    sprite: Sprite {
                        color: board_assets.tile_material.color,
                        custom_size: Some(Vec2::splat(size - padding as f32)),
                        ..Default::default()
                    },
                    transform: Transform::from_xyz(
                        (x as f32 * size) + (size / 2.0),
                        (y as f32 * size) + (size / 2.0),
                        1.0,
                    ),
                    texture: board_assets.tile_material.texture.clone(),
                    ..Default::default()
                })
                .insert(Name::new(format!("Tile ({}, {})", x, y)))
                .insert(coordinates);

                cmd.with_children(|parent| {
                    let entity = parent
                        .spawn_bundle(SpriteBundle {
                            sprite: Sprite {
                                custom_size: Some(Vec2::splat(size - padding)),
                                color: board_assets.covered_tile_material.color,
                                ..Default::default()
                            },
                            texture: board_assets.covered_tile_material.texture.clone(),
                            transform: Transform::from_xyz(0.0, 0., 2.0),
                            ..Default::default()
                        })
                        .insert(Name::new("Tile Cover"))
                        .id();
                    covered_tiles.insert(coordinates, entity);
                    if safe_start_entity.is_none() && *tile == Tile::Empty {
                        *safe_start_entity = Some(entity);
                    }
                });
                
                match tile {
                    Tile::Bomb => {
                        cmd.insert(Bomb);
                        cmd.with_children(|parent| {
                            parent.spawn_bundle(SpriteBundle {
                                sprite: Sprite {
                                    custom_size: Some(Vec2::splat(size - padding)),
                                    ..Default::default()
                                },
                                transform: Transform::from_xyz(0.0, 0.0, 1.0),
                                texture: board_assets.bomb_material.texture.clone(),
                                ..Default::default()
                            });
                        });
                    }
                    Tile::BombNeighbor(v) => {
                        cmd.insert(BombNeighbor{ count: *v });
                        cmd.with_children(|parent| {
                            parent.spawn_bundle(Self::bomb_count_text_bundle(*v, board_assets, size - padding));
                        });
                    }
                    Tile::Empty => ()
                }
            }
        }
    }

    fn adaptative_tile_size(window: Res<WindowDescriptor>, (min, max): (f32, f32), (width, height): (u16, u16)) -> f32 {
        let max_width = window.width / width as f32;
        let max_height = window.height / height as f32;

        max_width.min(max_height).clamp(min, max)
    }

    fn cleanup_board(board: Res<Board>, mut commands: Commands) {
        commands.entity(board.entity).despawn_recursive();
        commands.remove_resource::<Board>();
    }
}