pub mod components;
pub mod resources;

#[cfg(feature = "debug")]
use bevy_inspector_egui::{RegisterInspectable, InspectableRegistry};

use bevy::{log, prelude::*};
use resources::{tile_map::TileMap, BoardOptions, tile::Tile};

use crate::{resources::{TileSize, BoardPosition}, components::{Coordinates, BombNeighbor, Bomb}};

pub struct BoardPlugin;

impl Plugin for BoardPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(Self::create_board);
        log::info!("Loaded Board Plugin");

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
    }
}

impl BoardPlugin {
    pub fn create_board(
        mut commands: Commands,
        board_options: Option<Res<BoardOptions>>,
        window: Res<WindowDescriptor>,
        asset_server: Res<AssetServer>
    ) {
        let options = match board_options {
            None => BoardOptions::default(),
            Some(o) => o.clone(),
        };
        let mut tile_map = TileMap::empty(options.map_size.0, options.map_size.1);

        let font: Handle<Font> = asset_server.load("fonts/pixeled.ttf");
        let bomb_image:Handle<Image> = asset_server.load("sprites/bomb.png");

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
        log::info!("Board size: {}", board_size);

        let board_position = match options.position {
            BoardPosition::Centered { offset } => { Vec3::new(
                -(board_size.x / 2.0) ,
                -(board_size.y / 2.0),
                0.0) + offset
            }
            BoardPosition::Custom(p) => p,
        };

        tile_map.set_bombs(40);
        
        commands.spawn()
            .insert(Name::new("Board"))
            .insert(Transform::from_translation(board_position))
            .insert(GlobalTransform::default())
            .with_children(|parent| {
                parent
                    .spawn_bundle(SpriteBundle {
                        sprite: Sprite {
                            color: Color::WHITE,
                            custom_size: Some(board_size),
                            ..Default::default()
                        },
                        transform: Transform::from_xyz(board_size.x / 2.0, board_size.y / 2.0, 0.0),
                        ..Default::default()
                    })
                    .insert(Name::new("Background"));
                Self::spawn_tiles(parent, &tile_map, tile_size, options.tile_padding, Color::GRAY, bomb_image, font)
            });

        #[cfg(feature = "debug")]
        log::info!("{}", tile_map.console_output());
    }

    fn bomb_count_text_bundle(count: u8, font: Handle<Font>, size: f32) -> Text2dBundle {
        let (text, color) = (
            count.to_string(),
            match count {
                1 => Color::WHITE,
                2 => Color::GREEN,
                3 => Color::YELLOW,
                4 => Color::ORANGE,
                _ => Color::PURPLE,
            },
        );

        Text2dBundle {
            text: Text {
                sections: vec![TextSection {
                    value: text,
                    style: TextStyle {
                        font,
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
        color: Color,
        bomb_image: Handle<Image>,
        font: Handle<Font>,
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
                        color,
                        custom_size: Some(Vec2::splat(size - padding as f32)),
                        ..Default::default()
                    },
                    transform: Transform::from_xyz(
                        (x as f32 * size) + (size / 2.0),
                        (y as f32 * size) + (size / 2.0),
                        1.0,
                    ),
                    ..Default::default()
                })
                .insert(Name::new(format!("Tile ({}, {})", x, y)))
                .insert(coordinates);
                
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
                                texture: bomb_image.clone(),
                                ..Default::default()
                            });
                        });
                    }
                    Tile::BombNeighbor(v) => {
                        cmd.insert(BombNeighbor{ count: *v });
                        cmd.with_children(|parent| {
                            parent.spawn_bundle(Self::bomb_count_text_bundle(*v, font.clone(), size - padding));
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
}