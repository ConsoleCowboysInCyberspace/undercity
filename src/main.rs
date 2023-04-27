#![allow(unused, non_snake_case, non_upper_case_globals)]

use std::ops::Deref;

use bevy::math::{uvec2, vec2};
use bevy::prelude::*;
use bevy::time::TimePlugin;
use rand::Rng;

#[derive(Clone, Copy, Debug, Component)]
struct IsoTransform {
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
		let pos = vec2(iy - ix, (iy + ix) / 2.0);
		transform.translation = (pos, 10_000.0 - pos.y).into();
	}
}

#[derive(Clone, Copy, Debug)]
enum Tile {
	Floor,
	WallSolid,

	Random,
}

impl Tile {
	fn atlas_index(self) -> UVec2 {
		match self {
			Tile::Floor => uvec2(4, 2),
			Tile::WallSolid => uvec2(7, 1),
			Tile::Random => {
				let v = rand::thread_rng().gen_range(0 .. 21);
				uvec2(v % 8, v / 8)
			},
		}
	}
}

struct Map {
	size: UVec2,
	tiles: Vec<Tile>,
}

impl Map {
	fn new(size: UVec2) -> Self {
		Self {
			size,
			tiles: vec![Tile::Floor; (size.x * size.y) as _],
		}
	}

	fn into_tiles(self, atlas: Handle<Image>) -> impl Iterator<Item = (Vec2, SpriteBundle)> {
		let size = self.size;
		self.tiles
			.into_iter()
			.enumerate()
			.map(move |(index, tile)| {
				let pos = vec2((index as u32 % size.x) as _, (index as u32 / size.x) as _);
				let atlasIndex = tile.atlas_index().as_vec2() * 64.0;
				(
					pos,
					SpriteBundle {
						texture: atlas.clone(),
						sprite: Sprite {
							rect: Some(Rect::new(
								atlasIndex.x,
								atlasIndex.y,
								atlasIndex.x + 64.0,
								atlasIndex.y + 64.0,
							)),
							..default()
						},
						..default()
					},
				)
			})
	}
}

fn main() {
	let mut app = App::new();

	app.add_plugins(DefaultPlugins);

	app.add_system(isotransform_update_system);
	app.add_startup_system(|mut cmd: Commands, assets: ResMut<AssetServer>| {
		cmd.spawn(Camera2dBundle {
			transform: Transform::from_xyz(0.0, 0.0, 10_000.0),
			projection: OrthographicProjection {
				far: 20_000.0,
				..default()
			},
			..default()
		});

		let atlas = assets.load("tiles/cocutos.png");

		let mut map = Map::new(UVec2::splat(16));
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

		for (pos, sprite) in map.into_tiles(atlas.clone()) {
			cmd.spawn((IsoTransform { pos, scale: 32.0 }, sprite))
				.with_children(|b| {
					let ai = Tile::Floor.atlas_index().as_vec2() * 64.0;
					b.spawn(SpriteBundle {
						texture: atlas.clone(),
						sprite: Sprite {
							rect: Some(Rect::new(ai.x, ai.y, ai.x + 64.0, ai.y + 64.0)),
							..default()
						},
						transform: Transform::from_xyz(0.0, 0.0, -0.5),
						..default()
					});
				});
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
