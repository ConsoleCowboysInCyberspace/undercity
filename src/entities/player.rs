use bevy::math::{vec2, Vec3Swizzles};
use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use rand::{thread_rng, Rng};

use crate::map::{tileDiameter, tileRadius};
use crate::{iso_pos, IsoSprite, IsoSpriteBundle};

pub const depthRange: f32 = 1_000_000.0;

#[derive(Component)]
struct Player;

#[linkme::distributed_slice(crate::setupApp)]
fn setup_app(app: &mut App) {
	app.add_startup_system(startup);
	app.add_system(move_player);
	app.add_system(move_camera.after(move_player));
	app.add_system(zoom_camera);
}

fn startup(mut cmd: Commands, assets: Res<AssetServer>) {
	let spritePos = vec2(256.0, 448.0);
	cmd.spawn((
		Player,
		IsoSpriteBundle {
			sprite: IsoSprite {
				texture: assets.load("tiles/misc.png"),
				rect: Rect {
					min: spritePos,
					max: spritePos + 64.0,
				},
				flip: false,
			},
			// FIXME: mapgen needs to set this
			transform: Transform::from_translation(
				(vec2(2.0, -2.0) * crate::map::tileRadius, 0.0).into(),
			)
			.into(),
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

	let (mut controller, mut sprite) = playerQuery.single_mut();
	let displacement = vel.normalize_or_zero() * tileDiameter * time.delta_seconds();
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
	mut playerQuery: Query<&Transform, With<Player>>,
	mut cameraQuery: Query<&mut Transform, (With<Camera2d>, Without<Player>)>,
) {
	let mut pos = iso_pos(playerQuery.single().translation.xy());
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
