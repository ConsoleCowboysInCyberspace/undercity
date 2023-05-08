#![allow(unused, non_snake_case, non_upper_case_globals)]

pub mod entities;
pub mod map;

use std::ops::Deref;

use bevy::math::{uvec2, vec2, Vec3Swizzles};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::render::{Extract, RenderApp};
use bevy::sprite::{ExtractedSprite, ExtractedSprites};
use bevy::time::TimePlugin;
use bevy::window::close_on_esc;
use rand::seq::SliceRandom;
use rand::{thread_rng, Rng};

use self::entities::player::depthRange;

#[linkme::distributed_slice]
pub static setupApp: [fn(&mut App)] = [..];

#[derive(Clone, Debug, Default, Component)]
pub struct IsoSprite {
	pub texture: Handle<Image>,
	pub rect: Rect,
	pub flip: bool,
}

#[derive(Debug, Default, Bundle)]
pub struct IsoSpriteBundle {
	pub sprite: IsoSprite,

	#[bundle]
	pub transform: TransformBundle,

	#[bundle]
	pub visibility: VisibilityBundle,
}

pub fn iso_pos(pos: Vec2) -> Vec3 {
	let (ix, iy) = pos.into();
	let pos = vec2(ix + iy, (ix - iy) / 2.0);
	(pos, depthRange / 2.0 - pos.y).into()
}

pub fn isosprite_extract(
	mut query: Extract<Query<(Entity, &GlobalTransform, &IsoSprite)>>,
	mut extractedSprites: ResMut<ExtractedSprites>,
) {
	for (entity, transform, sprite) in query.iter() {
		let mut affine = transform.affine();
		let mut isoPos = iso_pos(affine.translation.xy());
		isoPos.z += affine.translation.z;
		affine.translation = isoPos.into();
		extractedSprites.sprites.push(ExtractedSprite {
			entity,
			transform: GlobalTransform::from(affine),
			color: Color::WHITE,
			rect: Some(sprite.rect),
			custom_size: None,
			image_handle_id: sprite.texture.id(),
			flip_x: sprite.flip,
			flip_y: false,
			anchor: Vec2::ZERO, // center
		});
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

	app.sub_app_mut(RenderApp)
		.add_system(isosprite_extract.in_schedule(ExtractSchedule));

	app.add_system(close_on_esc);
	// app.add_system(isotransform_update_system);
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
