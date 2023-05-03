#![allow(unused, non_snake_case, non_upper_case_globals)]

pub mod entities;
pub mod map;

use std::ops::Deref;

use bevy::math::{uvec2, vec2};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::time::TimePlugin;
use bevy::window::close_on_esc;
use rand::seq::SliceRandom;
use rand::{thread_rng, Rng};

#[linkme::distributed_slice]
pub static setupApp: [fn(&mut App)] = [..];

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

	for func in setupApp {
		func(&mut app);
	}

	app.add_system(close_on_esc);
	app.add_system(isotransform_update_system);
	app.add_startup_system(|mut cmd: Commands, assets: ResMut<AssetServer>| {
		use map::*;
		let mut map = Map::new();

		let wall = Tile {
			ty: TileType::Wall(WallShape::Solid),
			tileset: Tileset::Cocutos,
		};
		for y in -4 .. 5 {
			for x in [-4, 4] {
				map[(x, y)].set(wall);
			}
		}
		for x in -4 .. 5 {
			for y in [-4, 4] {
				map[(x, y)].set(wall);
			}
		}
		for y in -3 .. 4 {
			for x in -3 .. 4 {
				let lava = (-1 ..= 1);
				map[(x, y)].set(Tile {
					ty: TileType::Floor(if x == 0 && y == 0 {
						FloorType::LavaBlue
					} else if lava.contains(&x) && lava.contains(&y) {
						FloorType::LavaRed
					} else {
						FloorType::Tileset
					}),
					tileset: wall.tileset,
				});
			}
		}

		map.into_entities(&mut cmd, &assets);
	});

	app.run();
}
