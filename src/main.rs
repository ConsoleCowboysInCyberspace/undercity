#![allow(unused, non_snake_case, non_upper_case_globals)]

pub mod entities;
pub mod map;

use std::ops::Deref;

use bevy::math::{ivec2, uvec2, vec2, vec3, Affine3A, Vec3Swizzles};
use bevy::prelude::*;
use bevy::render::extract_component::ExtractComponent;
use bevy::render::render_resource::{Extent3d, FilterMode, TextureDimension, TextureFormat};
use bevy::render::{Extract, RenderApp, RenderSet};
use bevy::sprite::{ExtractedSprite, ExtractedSprites};
use bevy::time::TimePlugin;
use bevy::window::close_on_esc;
use bevy_rapier2d::prelude::RapierPhysicsPlugin;
use bevy_rapier2d::render::{DebugRenderContext, RapierDebugRenderPlugin};
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
	time: Res<Time>,
	mut last: Local<f32>,
) {
	for (entity, transform, sprite) in query.iter() {
		let mut affine = transform.affine();
		let mut isoPos = iso_pos(affine.translation.xy());
		isoPos.z += affine.translation.z;
		affine.translation = isoPos.into();
		extractedSprites.sprites.push(ExtractedSprite {
			entity,
			transform: affine.into(),
			color: Color::WHITE,
			rect: Some(sprite.rect),
			custom_size: None,
			image_handle_id: sprite.texture.id(),
			flip_x: sprite.flip,
			flip_y: false,
			anchor: Vec2::ZERO, // center
		});
	}

	let now = time.elapsed_seconds();
	if now - *last > 1.0 {
		*last = now;
		eprintln!("{} sprites", extractedSprites.sprites.len());
	}
}

fn main() {
	let mut app = App::new();

	app.add_plugins(
		DefaultPlugins
			.set(WindowPlugin {
				primary_window: Some(Window {
					title: "The Undercity".into(),
					resolution: (1920.0, 1080.0).into(),
					..default()
				}),
				..default()
			})
			.set(ImagePlugin {
				default_sampler: bevy::render::render_resource::SamplerDescriptor {
					// use nearest neighbor when scaling up textures, for the  a e s t h e t i c
					mag_filter: FilterMode::Nearest,
					// but still linear when scaling down, to help suppress Moiré patterns
					min_filter: FilterMode::Linear,
					mipmap_filter: FilterMode::Linear,
					..default()
				}
			}),
	);

	app.add_plugin(RapierPhysicsPlugin::<()>::pixels_per_meter(
		crate::map::tileDiameter,
	));
	app.add_plugin(
		RapierDebugRenderPlugin::default()
			.always_on_top()
			.disabled(),
	);
	app.add_system(toggle_rapier_debug);

	for func in setupApp {
		func(&mut app);
	}

	app.sub_app_mut(RenderApp)
		.add_system(isosprite_extract.in_schedule(ExtractSchedule));

	app.add_system(close_on_esc);
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
					..wall
				});
			}
		}

		map[(0, 0)].set(Tile {
			ty: TileType::Wall(WallShape::Pillar),
			tileset: Tileset::Lapis,
		});
		map[(0, -4)].set(Tile {
			ty: TileType::Floor(FloorType::Tileset),
			..wall
		});

		let shapes = [
			WallShape::Pillar,
			WallShape::North,
			WallShape::East,
			WallShape::South,
			WallShape::West,
			WallShape::Northeast,
			WallShape::Northwest,
			WallShape::Southeast,
			WallShape::Southwest,
			WallShape::Eastwest,
			WallShape::Northsouth,
			WallShape::Solid,
			WallShape::SolidNorth,
			WallShape::SolidEast,
			WallShape::SolidSouth,
			WallShape::SolidWest,
		];
		for (x, shape) in shapes.into_iter().enumerate() {
			let x = x as i32 - (shapes.len() / 2) as i32;
			map[(x, -8)].set(Tile {
				ty: TileType::Wall(shape),
				tileset: Tileset::BrickCyan,
			});
		}

		let mut room = Map::new();
		let tileset = Tileset::Gehena;
		room[TilePos::of(0, 0)].set(Tile {
			ty: TileType::Wall(WallShape::Southeast),
			tileset,
		});
		room[TilePos::of(1, 0)].set(Tile {
			ty: TileType::Wall(WallShape::South),
			tileset,
		});
		room[TilePos::of(2, 0)].set(Tile {
			ty: TileType::Wall(WallShape::Southwest),
			tileset,
		});
		room[TilePos::of(0, 1)].set(Tile {
			ty: TileType::Wall(WallShape::East),
			tileset,
		});
		room[TilePos::of(1, 1)].set(Tile {
			// ty: TileType::Wall(WallShape::Pillar),
			ty: TileType::Wall(WallShape::Solid),
			tileset,
		});
		room[TilePos::of(2, 1)].set(Tile {
			ty: TileType::Wall(WallShape::West),
			tileset,
		});
		room[TilePos::of(0, 2)].set(Tile {
			ty: TileType::Wall(WallShape::Northeast),
			tileset,
		});
		room[TilePos::of(1, 2)].set(Tile {
			ty: TileType::Wall(WallShape::North),
			tileset,
		});
		room[TilePos::of(2, 2)].set(Tile {
			ty: TileType::Wall(WallShape::Northwest),
			tileset,
		});
		room.fill_border(
			Tile {
				ty: TileType::Wall(WallShape::Solid),
				tileset,
			},
			TilePos::of(-1, -1),
			TilePos::of(3, 3),
		);
		map.copy_from(&room, TilePos::of(-10, -1));
		map.copy_from(&room, TilePos::of(-10, -10));

		map.into_entities(&mut cmd, &assets);
	});

	app.run();
}

fn toggle_rapier_debug(keyboard: Res<Input<KeyCode>>, mut ctx: ResMut<DebugRenderContext>) {
	if keyboard.just_pressed(KeyCode::F11) {
		ctx.enabled = !ctx.enabled;
	}
}
