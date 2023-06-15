use bevy::input::mouse::MouseMotion;
use bevy::math::{vec2, Vec3Swizzles};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_rapier2d::prelude::*;
use rand::{thread_rng, Rng};

use crate::map::{tileDiameter, tileRadius, Landmark, Tile, TileType};
use crate::{world_to_iso, IsoSprite, IsoSpriteBundle};

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
		Collider::ball(tileRadius / 5.0),
		ColliderDebugColor(Color::YELLOW),
		KinematicCharacterController {
			autostep: None,
			snap_to_ground: None,
			..default()
		},
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
	mut playerQuery: Query<(&mut KinematicCharacterController, &mut IsoSprite), With<Player>>,
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

	let (mut controller, mut sprite) = playerQuery.single_mut();
	let displacement = vel.normalize_or_zero() * tileDiameter * sprint * time.delta_seconds();
	controller.translation = Some(displacement);

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
