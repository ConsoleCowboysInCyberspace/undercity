use std::f32::consts::PI;
use std::fmt::Write;

use bevy::input::mouse::MouseWheel;
use bevy::math::{vec2, vec3, Vec3Swizzles};
use bevy::prelude::*;
use bevy::render::primitives::Aabb;
use bevy::window::PrimaryWindow;
use bevy_rapier2d::prelude::*;
use rand::seq::SliceRandom;
use rand::{thread_rng, Rng};

use super::Health;
use crate::map::{tileDiameter, tileRadius, FloorType, Landmark, MutMap, Tile, TileType};
use crate::{find_interactible_entities, world_to_iso, InteractEvent, IsoSprite, IsoSpriteBundle};

pub const depthRange: f32 = 1_000_000.0;

#[derive(Component)]
pub struct Player;

#[derive(Component)]
pub struct Cursor;

#[linkme::distributed_slice(crate::setupApp)]
fn setup_app(app: &mut App) {
	app.add_startup_systems((startup, startup_gui));
	app.add_systems((
		move_player,
		move_camera.after(move_player),
		zoom_camera,
		move_cursor,
		interact.after(move_cursor),
		#[cfg(debug_assertions)]
		teleport,
		update_gui,
	));
}

#[linkme::distributed_slice(crate::setupMap)]
fn setup_map(map: &mut MutMap, cmd: &mut Commands, assets: &AssetServer) {
	let playerSpawns = map.pluck_tiles(TileType::Floor(FloorType::Tileset), |_, pair| {
		matches!(
			pair.foreground.ty,
			TileType::Landmark {
				ty: Landmark::SpawnPlayer,
				..
			}
		)
	});
	// FIXME: use map rng
	let playerSpawn = playerSpawns.choose(&mut thread_rng()).unwrap().0;
	cmd.add(move |world: &mut World| {
		let mut query = world.query_filtered::<&mut Transform, With<Player>>();
		query.single_mut(world).translation = (playerSpawn.as_vec2() * tileRadius, 0.0).into();
	});
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
		Health::new(100.0),
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
	mut mouseWheel: EventReader<MouseWheel>,
) {
	const step: f32 = 0.1;
	const min: f32 = 0.1;
	const max: f32 = 2.5;

	let mut scrollDelta = 0.0;
	for ev in &mut mouseWheel {
		scrollDelta += ev.y;
	}

	let add = if keyboard.just_pressed(KeyCode::Equals) || scrollDelta > 0.0 {
		-step
	} else if keyboard.just_pressed(KeyCode::Minus) || scrollDelta < 0.0 {
		step
	} else {
		return;
	};
	let mut projection = query.single_mut();
	projection.scale = (projection.scale.ln() + add).exp().clamp(min, max);
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
	const reachDistance: f32 = tileDiameter * 0.75;

	let keyboard: &Input<KeyCode> = world.resource();
	if keyboard.just_pressed(KeyCode::E) {
		let mut cursor = world.query_filtered::<&Transform, With<Cursor>>();
		let mut player = world.query_filtered::<(Entity, &Transform), With<Player>>();

		let cursorPos = cursor.single(&world).translation.xy();
		let (player, plyPos) = player.single(&world);
		let plyPos = plyPos.translation.xy();

		if cursorPos.distance_squared(plyPos) > reachDistance.powf(2.0) {
			return;
		}

		let ents = find_interactible_entities(cursorPos, 8.0, world);
		let Some(&target) = ents.first() else { return; };
		world
			.entity_mut(target)
			.insert(InteractEvent { source: player });
	}
}

#[cfg(debug_assertions)]
fn teleport(
	mut player: Query<&mut Transform, With<Player>>,
	keyboard: Res<Input<KeyCode>>,
	mut savedPos: Local<Option<Vec2>>,
) {
	if keyboard.just_pressed(KeyCode::Y) {
		*savedPos = Some(player.single().translation.xy());
		eprintln!("saved position {:?}", savedPos.unwrap());
	}

	if keyboard.just_pressed(KeyCode::T) {
		if let Some(pos) = *savedPos {
			let translation = &mut player.single_mut().translation;
			*translation = (pos, translation.z).into();
			eprintln!("recalled position {:?}", savedPos.unwrap());
		}
	}
}

#[derive(Component)]
struct HealthBarRect;

#[derive(Component)]
struct HealthBarText;

fn startup_gui(mut cmd: Commands, assets: Res<AssetServer>) {
	let (width, height) = (200.0, 50.0);

	cmd.spawn(NodeBundle {
		style: Style {
			size: Size::new(Val::Px(width), Val::Px(height)),
			align_items: AlignItems::Center,
			justify_content: JustifyContent::Center,
			position_type: PositionType::Absolute,
			position: UiRect {
				bottom: Val::Px(10.0),
				left: Val::Px(10.0),
				..default()
			},
			..default()
		},
		background_color: BackgroundColor(Color::GRAY),
		..default()
	})
	.with_children(|parent| {
		parent.spawn((
			HealthBarRect,
			NodeBundle {
				style: Style {
					size: Size::new(Val::Px(width), Val::Px(height - 5.0)),
					position_type: PositionType::Absolute,
					position: UiRect {
						top: Val::Px(2.5),
						left: Val::Percent(0.0),
						..default()
					},
					..default()
				},
				background_color: BackgroundColor(Color::RED),
				..default()
			},
		));

		parent.spawn((
			HealthBarText,
			TextBundle {
				text: Text::from_section(
					"-",
					TextStyle {
						font: assets.load("fonts/RedHatDisplay.ttf"),
						font_size: 48.0,
						color: Color::WHITE,
					},
				),
				style: Style {
					size: Size::new(Val::Auto, Val::Px(height)),
					position_type: PositionType::Absolute,
					position: UiRect {
						top: Val::Percent(0.0),
						..default()
					},
					align_self: AlignSelf::Center,
					..default()
				},
				..default()
			},
		));
	});
}

fn update_gui(
	health: Query<&Health, (With<Player>, Changed<Health>)>,
	mut rect: Query<&mut Style, With<HealthBarRect>>,
	mut text: Query<&mut Text, With<HealthBarText>>,
	time: Res<Time>,
) {
	for &Health(health) in &health {
		rect.single_mut().size.width = Val::Percent(health);

		let text = &mut text.single_mut().sections[0].value;
		text.clear();
		write!(text, "{health:.0}").unwrap();
	}
}
