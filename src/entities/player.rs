use bevy::math::{vec2, Vec3Swizzles};
use bevy::prelude::*;
use rand::{thread_rng, Rng};

use crate::map::tileDiameter;
use crate::{iso_pos, IsoSprite, IsoSpriteBundle, IsoTransform};

pub const depthRange: f32 = 1_000_000.0;

#[derive(Component)]
struct Player;

#[linkme::distributed_slice(crate::setupApp)]
fn setup_app(app: &mut App) {
	app.add_startup_system(startup);
	app.add_system(move_player);
	app.add_system(move_camera.after(move_player));
}

fn startup(mut cmd: Commands, assets: Res<AssetServer>) {
	let spritePos = vec2(256.0, 448.0);
	cmd.spawn((
		Player,
		IsoSpriteBundle {
			isoTransform: IsoTransform { scale: 1.0 },
			sprite: IsoSprite {
				texture: assets.load("tiles/misc.png"),
				rect: Rect {
					min: spritePos,
					max: spritePos + 64.0,
				},
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
	mut playerQuery: Query<(&mut Transform, &mut IsoSprite), With<Player>>,
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

	let (mut transform, mut sprite) = playerQuery.single_mut();
	let displacement = vel.normalize_or_zero() * tileDiameter * time.delta_seconds();
	transform.translation += Vec3::from((displacement, 0.0));

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
	mut playerQuery: Query<&Transform, With<Player>>,
	mut cameraQuery: Query<&mut Transform, (With<Camera2d>, Without<Player>)>,
) {
	let mut pos = iso_pos(playerQuery.single().translation.xy(), 1.0);
	pos.z = depthRange;
	cameraQuery.single_mut().translation = pos;
}
