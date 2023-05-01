#![allow(unused, non_snake_case, non_upper_case_globals)]

pub mod map;

use std::ops::Deref;

use bevy::math::{uvec2, vec2};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::time::TimePlugin;
use bevy::window::close_on_esc;
use rand::Rng;

#[derive(Clone, Copy, Debug, Component)]
pub struct IsoTransform {
	pos: Vec2,
	scale: f32,
}

impl Default for IsoTransform {
	fn default() -> Self {
		Self {
			pos: Vec2::ZERO,
			scale: 1.0,
		}
	}
}

fn isotransform_update_system(
	mut query: Query<
		(Entity, &mut Transform, &IsoTransform),
		Or<(Added<IsoTransform>, Changed<IsoTransform>)>,
	>,
) {
	for (ent, mut transform, isoTransform) in query.iter_mut() {
		let (ix, iy) = (isoTransform.pos * isoTransform.scale).into();
		let pos = vec2(ix + iy, (ix - iy) / 2.0);
		transform.translation = (pos, 500_000.0 - pos.y).into();
	}
}

fn main() {
	let mut app = App::new();

	app.add_plugins(DefaultPlugins.set(WindowPlugin {
		primary_window: Some(Window {
			title: "The Undercity".into(),
			resolution: (1920.0, 1080.0).into(),
			..default()
		}),
		..default()
	}));

	app.add_system(close_on_esc);
	app.add_system(isotransform_update_system);
	app.add_startup_system(|mut cmd: Commands, assets: ResMut<AssetServer>| {
		cmd.spawn(Camera2dBundle {
			transform: Transform::from_xyz(0.0, 0.0, 1_000_000.0),
			projection: OrthographicProjection {
				far: 1_000_000.0,
				..default()
			},
			..default()
		});

		use map::*;
		let mut map = Map::new(UVec2::splat(32));

		#[cfg(none)]
		for y in 0 .. map.size.y {
			for x in 0 .. map.size.x {
				let edge = y == 0 || x == 0 || y == map.size.y - 1 || x == map.size.x - 1;
				let tile = if edge {
					Tile::WallSolid
				} else {
					let interior = y > 1 && y < map.size.y - 2 && x > 1 && x < map.size.x - 2;
					if interior {
						Tile::Random
					} else {
						Tile::Floor
					}
				};

				let index = y * map.size.x + x;
				map.tiles[index as usize] = tile;
			}
		}

		map.tiles[0] = Tile {
			tile: TileType::Floor(FloorType::Tileset),
			..default()
		};
		map.tiles[2] = Tile {
			tile: TileType::Floor(FloorType::LavaBlue),
			..default()
		};
		map.tiles[4] = Tile {
			tile: TileType::Landmark(Landmark::ShrineIdol),
			..default()
		};

		for (pos, tile) in map.into_tiles() {
			cmd.spawn(tile.into_bundle(pos, &*assets));
		}
	});

	app.add_system(
		|mut query: Query<&mut Transform, With<Camera2d>>,
		 time: Res<Time>,
		 keyboard: Res<Input<KeyCode>>| {
			let mut vel = Vec2::ZERO;
			if keyboard.pressed(KeyCode::W) {
				vel.y += 1.0;
			}
			if keyboard.pressed(KeyCode::S) {
				vel.y -= 1.0;
			}
			if keyboard.pressed(KeyCode::A) {
				vel.x -= 1.0;
			}
			if keyboard.pressed(KeyCode::D) {
				vel.x += 1.0;
			}

			let mut transform = query.single_mut();
			transform.translation +=
				Vec3::from((vel.normalize_or_zero() * 250.0 * time.delta_seconds(), 0.0));
		},
	);

	app.run();
}
