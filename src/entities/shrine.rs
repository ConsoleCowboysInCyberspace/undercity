use bevy::prelude::*;
use bevy_rapier2d::prelude::{Collider, RigidBody};

use super::player::Player;
use super::Health;
use crate::map::{tileRadius, FloorType, Landmark, Map, MutMap, TileType};
use crate::{InteractEvent, Interactible, IsoSpriteBundle};

#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub enum ShrineType {
	Heal,
	Damage,
	// Blink, // requires keeping `Map`s around?
}

impl From<Landmark> for ShrineType {
	fn from(landmark: Landmark) -> Self {
		match landmark {
			Landmark::ShrineIdol => Self::Heal,
			Landmark::ShrineSkulls => Self::Damage,
			// Landmark::ShrineScroll => Self::Blink,
			_ => panic!("no shrine for landmark {landmark:?}"),
		}
	}
}

#[derive(Component)]
pub struct Shrine(ShrineType);

#[linkme::distributed_slice(crate::setupApp)]
fn setup_app(app: &mut App) {
	app.add_system(handle_interactions);
}

#[linkme::distributed_slice(crate::setupMap)]
fn setup_map(map: &mut MutMap, cmd: &mut Commands, assets: &AssetServer) {
	const radius: f32 = tileRadius / 2.0 * 0.9;
	let collider = Collider::cuboid(radius, radius);

	let shrines = map.pluck_tiles(TileType::Floor(FloorType::Tileset), |_, pair| {
		use Landmark::*;
		matches!(
			pair.foreground.ty,
			TileType::Landmark {
				ty: ShrineIdol | ShrineSkulls | ShrineScroll,
				..
			}
		)
	});
	for (pos, tile) in shrines {
		let TileType::Landmark { ty: landmark, flip } = tile.ty else { unreachable!() };
		let (sprite, _) = tile.into_bundle(pos.as_vec2(), assets);
		cmd.spawn((
			Shrine(ShrineType::from(landmark)),
			sprite,
			Interactible,
			RigidBody::Fixed,
			collider.clone(),
		));
	}
}

fn handle_interactions(
	mut cmd: Commands,
	mut shrines: Query<(Entity, &InteractEvent, &Shrine), Added<InteractEvent>>,
	mut player: Query<&mut Health, With<Player>>,
	map: Res<Map>,
) {
	for (shrineEnt, ev, shrine) in &mut shrines {
		cmd.entity(shrineEnt).remove::<InteractEvent>();
		match shrine.0 {
			ShrineType::Heal => {
				player.single_mut().take_healing(25.0);
			},
			ShrineType::Damage => {
				player.single_mut().take_damage(10.0);
			},
			_ => todo!("new shrine type {:?}", shrine.0),
		}
	}
}
