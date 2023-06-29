use anyhow::anyhow;
use bevy::prelude::*;
use bevy_rapier2d::prelude::{Collider, CollisionGroups, Group, RigidBody, Sensor, SolverGroups};
use bevy_rapier2d::render::ColliderDebugColor;
use rand::seq::{IteratorRandom, SliceRandom};
use rand::thread_rng;

use crate::map::{Tile, TilePos, TileType, Tileset, WallShape};
use crate::{AResult, IsoSprite};

#[derive(Component)]
pub struct Door(Tile, Collider);

impl Door {
	pub fn is_open(&self) -> bool {
		matches!(
			self.0.ty,
			TileType::DoorNS { open } |
			TileType::DoorEW { open } if open
		)
	}

	pub fn toggle(&mut self) {
		match &mut self.0.ty {
			TileType::DoorNS { open } | TileType::DoorEW { open } => *open = !*open,
			_ => unreachable!(),
		}
	}
}

pub fn make_door(cmd: &mut Commands, assets: &AssetServer, pos: TilePos, tile: Tile) {
	assert!(matches!(
		tile.ty,
		TileType::DoorNS { .. } | TileType::DoorEW { .. }
	));
	let (sprite, _) = tile.into_bundle(pos.as_vec2(), assets);

	let shape = match tile.ty {
		TileType::DoorNS { .. } => WallShape::Northsouth,
		TileType::DoorEW { .. } => WallShape::Eastwest,
		_ => unreachable!(),
	};
	let collider = shape.collider();
	assert!(collider.translation.is_none());
	let collider = collider.collider;

	// #[cfg(none)]
	cmd.spawn((
		Door(tile, collider.clone()),
		sprite,
		RigidBody::Fixed,
		collider,
		CollisionGroups::default(),
		SolverGroups::default(),
	));
}

#[linkme::distributed_slice(crate::setupApp)]
fn setup_app(app: &mut App) {
	app.add_systems((update_doors, temp_toggle_doors));
}

fn update_doors(
	mut cmd: Commands,
	mut query: Query<
		(
			Entity,
			&Door,
			&mut IsoSprite,
			&mut CollisionGroups,
			&mut SolverGroups,
		),
		// Or<(Added<Door>, Changed<Door>)>
	>,
) {
	for (ent, door, mut sprite, mut collisionGroups, mut solverGroups) in query.iter_mut() {
		sprite.rect = door.0.texture_info().1;

		let mut ent = cmd.entity(ent);
		if door.is_open() {
			ent.insert(Sensor);
			// ent.insert(CollisionGroups::new(Group::ALL, Group::NONE));
			// ent.remove::<Collider>();
			ent.insert(ColliderDebugColor(Color::GREEN));
			collisionGroups.memberships = Group::NONE;
			collisionGroups.filters = Group::NONE;
			solverGroups.memberships = Group::NONE;
			solverGroups.filters = Group::NONE;
		} else {
			ent.remove::<Sensor>();
			// ent.insert(CollisionGroups::new(Group::ALL, Group::ALL));
			// ent.insert(door.1.clone());
			ent.insert(ColliderDebugColor(Color::RED));
			collisionGroups.memberships = Group::ALL;
			collisionGroups.filters = Group::ALL;
			solverGroups.memberships = Group::ALL;
			solverGroups.filters = Group::ALL;
		}
	}
}

fn temp_toggle_doors(
	mut query: Query<&mut Door>,
	time: Res<Time>,
	mut last: Local<f32>,
	mut indices: Local<Vec<usize>>,
	mut index: Local<usize>,
) {
	if query.is_empty() {
		return;
	}
	if indices.is_empty() {
		*indices = (0 .. query.iter().len()).collect();
	}
	if *index >= indices.len() {
		indices.shuffle(&mut thread_rng());
		*index = 0;
	}

	let mut door = query.iter_mut().nth(indices[*index]).unwrap();
	door.toggle();
	*index += 1;
}
