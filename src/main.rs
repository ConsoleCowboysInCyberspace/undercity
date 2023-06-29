#![allow(unused, non_snake_case, non_upper_case_globals)]

pub mod entities;
pub mod map;

use std::ops::Deref;

pub use anyhow::Result as AResult;
use bevy::log::LogPlugin;
use bevy::math::{ivec2, uvec2, vec2, vec3, Affine3A, Vec3Swizzles};
use bevy::prelude::*;
use bevy::render::extract_component::ExtractComponent;
use bevy::render::render_resource::{Extent3d, FilterMode, TextureDimension, TextureFormat};
use bevy::render::{Extract, RenderApp, RenderSet};
use bevy::sprite::{ExtractedSprite, ExtractedSprites, SpriteSystem};
use bevy::time::TimePlugin;
use bevy::window::close_on_esc;
use bevy_rapier2d::prelude::{RapierConfiguration, RapierPhysicsPlugin};
use bevy_rapier2d::render::{DebugRenderContext, RapierDebugRenderPlugin};
use rand::seq::SliceRandom;
use rand::{thread_rng, Rng};

use self::entities::player::{depthRange, Player};

#[linkme::distributed_slice]
pub static setupApp: [fn(&mut App)] = [..];

#[derive(Clone, Debug, Default, Component)]
pub struct IsoSprite {
	pub rect: Rect,
	pub flip: bool,
}

#[derive(Debug, Default, Bundle)]
pub struct IsoSpriteBundle {
	pub texture: Handle<Image>,

	pub sprite: IsoSprite,

	#[bundle]
	pub transform: TransformBundle,

	#[bundle]
	pub visibility: VisibilityBundle,
}

pub fn world_to_iso(pos: Vec2) -> Vec3 {
	let (ix, iy) = pos.into();
	let pos = vec2(ix + iy, (ix - iy) / 2.0);
	(pos, depthRange / 2.0 - pos.y).into()
}

pub fn iso_to_world(pos: Vec2) -> Vec2 {
	let (x, y) = pos.into();
	vec2(x / 2.0 + y, x / 2.0 - y)
}

pub fn isosprite_extract(
	mut query: Extract<Query<(Entity, &GlobalTransform, &Handle<Image>, &IsoSprite)>>,
	mut extractedSprites: ResMut<ExtractedSprites>,
	time: Res<Time>,
	mut last: Local<f32>,
) {
	for (entity, transform, texture, sprite) in query.iter() {
		let mut affine = transform.affine();
		let mut isoPos = world_to_iso(affine.translation.xy());
		isoPos.z += affine.translation.z;
		affine.translation = isoPos.into();
		extractedSprites.sprites.push(ExtractedSprite {
			entity,
			transform: affine.into(),
			color: Color::WHITE,
			rect: Some(sprite.rect),
			custom_size: None,
			image_handle_id: texture.id(),
			flip_x: sprite.flip,
			flip_y: false,
			anchor: Vec2::ZERO, // center
		});
	}

	let now = time.elapsed_seconds();
	#[cfg(none)]
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
				},
			})
			.set(LogPlugin {
				#[cfg(debug_assertions)]
				level: bevy::log::Level::DEBUG,
				filter: "wgpu=warn,naga=warn,bevy_ecs=info".into(),
				..default()
			}),
	);

	let mut rapierConfig = RapierConfiguration::default();
	rapierConfig.gravity = Vec2::ZERO;
	app.insert_resource(rapierConfig);
	app.add_plugin(RapierPhysicsPlugin::<()>::pixels_per_meter(
		crate::map::tileDiameter,
	));
	#[cfg(debug_assertions)]
	{
		app.add_plugin(
			RapierDebugRenderPlugin::default()
				.always_on_top()
				.disabled(),
		);
		app.add_system(toggle_rapier_debug);
	}

	for func in setupApp {
		func(&mut app);
	}

	app.sub_app_mut(RenderApp).add_system(
		isosprite_extract
			.after(SpriteSystem::ExtractSprites)
			.in_schedule(ExtractSchedule),
	);

	app.add_system(close_on_esc);
	app.add_startup_systems(
		(apply_system_buffers, setup_map)
			.chain()
			.after(entities::player::startup),
	);

	app.run();
}

fn setup_map(
	mut cmd: Commands,
	assets: Res<AssetServer>,
	mut playerQuery: Query<&mut Transform, With<Player>>,
) {
	use crate::map::*;

	let (mut map, mut rng) = map::gen::generate_map(0);

	let mut doors = map.pluck_tiles(TileType::Floor(FloorType::Tileset), |_, pair| {
		pair.is_door()
	});
	doors.extend(
		map.pluck_tiles(TileType::Floor(FloorType::Tileset), |_, pair| {
			pair.is_door()
		}),
	);
	for (pos, tile) in doors {
		entities::door::make_door(&mut cmd, &assets, pos, tile);
	}

	let playerSpawns = map.pluck_tiles(TileType::Floor(FloorType::Tileset), |_, pair| {
		matches!(
			pair.foreground.ty,
			TileType::Landmark {
				ty: Landmark::SpawnPlayer,
				..
			}
		)
	});
	let playerSpawn = playerSpawns.choose(&mut rng).unwrap().0;
	playerQuery.single_mut().translation = (playerSpawn.as_vec2() * tileRadius, 0.0).into();

	map.into_entities(&mut cmd, &assets);
}

fn toggle_rapier_debug(keyboard: Res<Input<KeyCode>>, mut ctx: ResMut<DebugRenderContext>) {
	if keyboard.just_pressed(KeyCode::F11) {
		ctx.enabled = !ctx.enabled;
	}
}
