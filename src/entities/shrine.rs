use bevy::math::Vec3Swizzles;
use bevy::prelude::*;
use bevy_rapier2d::prelude::{Collider, RigidBody};
use rand::{thread_rng, Rng};

use super::player::Player;
use super::Health;
use crate::map::{tileDiameter, tileRadius, FloorType, Landmark, Map, MutMap, TilePos, TileType};
use crate::{print_feed, InteractEvent, Interactible, IsoSpriteBundle};

#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub enum ShrineType {
	Heal,
	Damage,
	Blink,
}

impl From<Landmark> for ShrineType {
	fn from(landmark: Landmark) -> Self {
		match landmark {
			Landmark::ShrineIdol => Self::Heal,
			Landmark::ShrineSkulls => Self::Damage,
			Landmark::ShrineScroll => Self::Blink,
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

	let shrines = map.pluck_tiles(|_, pair| {
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
		let TileType::Landmark { ty: landmark, flip } = tile.ty else {
			unreachable!()
		};
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
	mut player: Query<(&mut Transform, &mut Health), With<Player>>,
	map: Res<Map>,
) {
	for (shrineEnt, ev, shrine) in &mut shrines {
		cmd.entity(shrineEnt).remove::<InteractEvent>();
		match shrine.0 {
			ShrineType::Heal => {
				print_feed!("The shrine heals you for 25 HP!");
				player.single_mut().1.take_healing(25.0);
			},
			ShrineType::Damage => {
				print_feed!("The shrine damages you for 10 HP!");
				player.single_mut().1.take_damage(10.0);
			},
			ShrineType::Blink => {
				print_feed!("The shrine drives you through the aether!");

				let transform = &mut player.single_mut().0;
				let usedTiles = map.used_tiles();
				let pos = {
					let x = thread_rng().gen_range(usedTiles.min.x ..= usedTiles.max.x);
					let y = thread_rng().gen_range(usedTiles.min.y ..= usedTiles.max.y);
					TilePos::of(x, y)
				};
				let Some(newPos) = map.find_tile(pos, |_, tile| tile.is_floor()) else {
					continue;
				};
				transform.translation =
					(newPos.as_vec2() * tileRadius, transform.translation.z).into();
			},
			_ => todo!("new shrine type {:?}", shrine.0),
		}
	}
}
