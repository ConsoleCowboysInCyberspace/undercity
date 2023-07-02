use bevy::input::mouse::MouseMotion;
use bevy::math::{vec2, vec3, Vec3Swizzles};
use bevy::prelude::*;
use bevy::render::primitives::Aabb;
use bevy::window::PrimaryWindow;
use bevy_rapier2d::prelude::*;
use rand::{thread_rng, Rng};

use crate::map::{tileDiameter, tileRadius, Landmark, Tile, TileType};
use crate::{find_interactible_entities, world_to_iso, InteractEvent, IsoSprite, IsoSpriteBundle};

pub const depthRange: f32 = 1_000_000.0;

#[derive(Component)]
pub struct Player;

#[derive(Component)]
pub struct Cursor;

#[linkme::distributed_slice(crate::setupApp)]
fn setup_app(app: &mut App) {
	app.add_startup_system(startup);
	app.add_systems((
		move_player,
		move_camera.after(move_player),
		zoom_camera,
		move_cursor,
		interact.after(move_cursor),
	));
}

pub fn startup(mut cmd: Commands, assets: Res<AssetServer>) {
	let (texture, playerRect, _) = Tile {
		ty: TileType::Landmark {
			ty: Landmark::SpawnPlayer,
			flip: false,
		},
		..default()
	}
	.texture_info();
	let texture = assets.load(texture);
	cmd.spawn((
		Player,
		IsoSpriteBundle {
			texture: texture.clone(),
			sprite: IsoSprite {
				rect: playerRect,
				flip: false,
			},
			..default()
		},
		RigidBody::Dynamic,
		LockedAxes::ROTATION_LOCKED,
		Dominance::group(64),
		Velocity::default(),
		Damping {
			linear_damping: 1.0,
			angular_damping: 1.0,
		},
		Collider::ball(tileRadius / 5.0),
		ColliderDebugColor(Color::YELLOW),
	));

	let (_, cursorRect, _) = Tile {
		ty: TileType::Landmark {
			ty: Landmark::Cursor,
			flip: false,
		},
		..default()
	}
	.texture_info();
	cmd.spawn((
		Cursor,
		IsoSpriteBundle {
			texture,
			sprite: IsoSprite {
				rect: cursorRect,
				flip: false,
			},
			..default()
		},
	));

	cmd.spawn(Camera2dBundle {
		projection: OrthographicProjection {
			far: depthRange,
			..default()
		},
		..default()
	});
}

fn move_player(
	mut playerQuery: Query<(&mut Velocity, &mut IsoSprite), With<Player>>,
	time: Res<Time>,
	keyboard: Res<Input<KeyCode>>,
	mut lastRngFlip: Local<f64>,
) {
	let mut vel = Vec2::ZERO;
	if keyboard.pressed(KeyCode::W) {
		vel.y -= 1.0;
	}
	if keyboard.pressed(KeyCode::S) {
		vel.y += 1.0;
	}
	if keyboard.pressed(KeyCode::A) {
		vel.x -= 1.0;
	}
	if keyboard.pressed(KeyCode::D) {
		vel.x += 1.0;
	}
	vel = vel.normalize_or_zero();

	let sprint = if keyboard.pressed(KeyCode::LShift) {
		4.0
	} else {
		1.0
	};

	let (mut velocity, mut sprite) = playerQuery.single_mut();
	velocity.linvel = vel.normalize_or_zero() * tileDiameter * sprint;

	// flip sprite to match movement direction
	if vel.length_squared() > 0.0 {
		let ne = vel.dot(vec2(-1.0, -1.0));
		let sw = vel.dot(vec2(1.0, 1.0));
		sprite.flip = if ne > 0.0 {
			false
		} else if sw > 0.0 {
			true
		} else {
			// no good orientation to pick, so randomly flip every 200ms
			const waitSecs: f64 = 0.2;
			let now = time.elapsed_seconds_f64();
			if now - *lastRngFlip > waitSecs {
				*lastRngFlip = now;
				thread_rng().gen_bool(0.5)
			} else {
				sprite.flip
			}
		};
	}
}

fn move_camera(
	playerQuery: Query<&Transform, With<Player>>,
	mut cameraQuery: Query<&mut Transform, (With<Camera2d>, Without<Player>)>,
) {
	let mut pos = world_to_iso(playerQuery.single().translation.xy());
	pos.z = depthRange;
	cameraQuery.single_mut().translation = pos;
}

fn zoom_camera(
	mut query: Query<&mut OrthographicProjection, With<Camera2d>>,
	keyboard: Res<Input<KeyCode>>,
) {
	const step: f32 = 0.1;

	let add = if keyboard.just_pressed(KeyCode::Equals) {
		-step
	} else if keyboard.just_pressed(KeyCode::Minus) {
		step
	} else {
		return;
	};
	let mut projection = query.single_mut();
	projection.scale = (projection.scale.ln() + add).exp();
}

fn move_cursor(
	mut cursor: Query<&mut Transform, With<Cursor>>,
	camera: Query<(&Camera, &GlobalTransform)>,
	window: Query<&Window, With<PrimaryWindow>>,
	mut lastPos: Local<Vec2>,
) {
	let mut pos = window.single().cursor_position().unwrap_or(*lastPos);
	*lastPos = pos;

	let (camera, transform) = camera.single();
	pos = camera.viewport_to_world_2d(transform, pos).unwrap();
	pos = crate::iso_to_world(pos);

	// snap to tile
	pos /= tileRadius;
	pos = vec2(pos.x.ceil(), pos.y.floor());
	pos *= tileRadius;

	cursor.single_mut().translation = (pos, tileDiameter).into();
}

fn interact(world: &mut World) {
	let keyboard: &Input<KeyCode> = world.resource();
	if keyboard.just_pressed(KeyCode::E) {
		let mut cursor = world.query_filtered::<&Transform, With<Cursor>>();
		let mut player = world.query_filtered::<Entity, With<Player>>();

		let cursorPos = cursor.single(&world).translation.xy();
		let player = player.single(&world);

		let ents = find_interactible_entities(cursorPos, 8.0, world);
		let Some(&target) = ents.first() else { return; };
		world.entity_mut(target).insert(InteractEvent {
			source: player,
		});
	}
}
