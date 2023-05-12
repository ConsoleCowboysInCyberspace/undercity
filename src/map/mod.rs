pub mod data;

use std::collections::HashMap;
use std::fmt::Debug;
use std::ops::{Deref, DerefMut, Index, IndexMut};

use bevy::math::{ivec2, uvec2, vec2};
use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use rand::{thread_rng, Rng};

pub use self::data::*;
use crate::{IsoSprite, IsoSpriteBundle};

pub const tileDiameter: f32 = 64.0;
pub const tileRadius: f32 = tileDiameter / 2.0;

#[derive(Clone, Copy, Debug, Default)]
pub struct Tile {
	pub ty: TileType,
	pub tileset: Tileset,
}

impl Tile {
	pub fn is_empty(&self) -> bool {
		matches!(self.ty, TileType::Empty)
	}

	pub fn texture_info(&self) -> (&'static str, Rect, bool) {
		let (texture, flip, index) = match self.ty {
			TileType::Empty => unimplemented!("Should never convert empty tiles into tile bundle"),
			TileType::Floor(floor) => (
				(!matches!(floor, FloorType::Tileset)).then_some("tiles/misc.png"),
				false,
				floor as _,
			),
			TileType::Wall(shape) => (None, false, shape as _),
			TileType::DoorNS { open } => (None, false, 17 + open.then_some(2).unwrap_or(0)),
			TileType::DoorEW { open } => (None, false, 16 + open.then_some(2).unwrap_or(0)),
			TileType::Landmark { ty, flip } => (Some("tiles/misc.png"), flip, ty as _),
		};
		let tilesetWidthElems = texture.map(|_| 16).unwrap_or(8);
		let index = uvec2(
			(index % tilesetWidthElems) as _,
			(index / tilesetWidthElems) as _,
		) * tileDiameter as u32;

		(
			texture.unwrap_or(self.tileset.asset_path()),
			Rect::new(
				index.x as f32,
				index.y as f32,
				index.x as f32 + tileDiameter,
				index.y as f32 + tileDiameter,
			),
			flip,
		)
	}

	pub fn into_bundle(
		self,
		pos: Vec2,
		assets: &AssetServer,
	) -> (IsoSpriteBundle, Option<PositionedCollider>) {
		let (texture, rect, flip) = self.texture_info();
		let texture = assets.load(texture);
		let collider = match self.ty {
			TileType::Wall(shape) => Some(shape.collider()),
			_ => None,
		};

		(
			IsoSpriteBundle {
				sprite: IsoSprite {
					texture,
					rect,
					flip,
					..default()
				},
				transform: Transform::from_translation((pos * tileRadius, 0.0).into()).into(),
				..default()
			},
			collider,
		)
	}
}

#[derive(Clone, Copy, Debug, Default)]
pub struct TilePair {
	pub foreground: Tile,
	pub background: Tile,
}

impl TilePair {
	pub fn is_empty(&self) -> bool {
		self.foreground.is_empty() && self.background.is_empty()
	}

	/// Set foreground or background depending on type of `tile`. Clears
	/// foreground if setting a floor.
	pub fn set(&mut self, tile: Tile) {
		if matches!(tile.ty, TileType::Floor(_)) {
			self.foreground.ty = TileType::Empty;
			self.background = tile;
		} else {
			self.set_with_floor(tile, FloorType::Tileset);
		}
	}

	/// Set a wall tile with a custom floor type in its background.
	pub fn set_with_floor(&mut self, tile: Tile, floor: FloorType) {
		debug_assert!(
			!matches!(tile.ty, TileType::Floor(_)),
			"TilePair::set_with_floor expects a wall tile"
		);
		self.foreground = tile;
		self.background = Tile {
			ty: TileType::Floor(floor),
			tileset: tile.tileset,
		};
	}

	pub fn clear(&mut self) {
		self.foreground.ty = TileType::Empty;
		self.background.ty = TileType::Empty;
	}

	pub fn into_entity(self, pos: TilePos, cmd: &mut Commands, assets: &AssetServer) -> Entity {
		debug_assert!(!self.is_empty(), "Attempting to spawn empty TilePair");

		let Self {
			foreground,
			background,
		} = self;
		let pos = pos.as_vec2();
		let mut foreground = if foreground.is_empty() {
			cmd.spawn((
				TransformBundle::from(Transform::from_translation((pos * tileRadius, 0.0).into())),
				VisibilityBundle::default(),
			))
		} else {
			let (foreground, collider) = foreground.into_bundle(pos, assets);
			let mut ent = cmd.spawn(foreground);
			if let Some(c) = collider {
				c.insert_into(&mut ent);
			}
			ent
		};

		if !background.is_empty() {
			let (background, _) = background.into_bundle(pos, assets);
			// FIXME: eventually floors will sometimes have (sensor) colliders, e.g. lava
			foreground.with_children(|b| {
				b.spawn(IsoSpriteBundle {
					// ensures players, mobs, etc. render over background
					transform: Transform::from_xyz(0.0, 0.0, -tileDiameter).into(),
					..background
				});
			});
		}

		foreground.id()
	}
}

#[derive(Clone)]
pub struct Chunk {
	pub pos: ChunkPos,
	pub tiles: [TilePair; Self::diameterTiles.pow(2)],
}

impl Chunk {
	pub const diameterTiles: usize = 32;

	pub fn new(pos: ChunkPos) -> Self {
		Self {
			pos,
			tiles: [default(); Self::diameterTiles.pow(2)],
		}
	}

	pub fn tile_positions(&self) -> impl Iterator<Item = TilePos> {
		let pos = self.pos;
		(0 .. Chunk::diameterTiles as i32).flat_map(move |y| {
			(0 .. Chunk::diameterTiles as i32)
				.map(move |x| TilePos::of(pos.x << 5 | x, pos.y << 5 | y))
		})
	}
}

impl Debug for Chunk {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Chunk")
			.field("pos", &self.pos)
			.field("tiles", &format!("<{} tiles>", self.tiles.len()))
			.finish()
	}
}

#[derive(Clone, Debug)]
pub struct Map {
	pub chunks: HashMap<ChunkPos, Chunk>,
}

impl Map {
	pub fn new() -> Self {
		Self {
			chunks: HashMap::new(),
		}
	}

	pub fn into_entities(self, cmd: &mut Commands, assets: &AssetServer) {
		let tiles = self
			.chunks
			.into_iter()
			.flat_map(|(pos, chunk)| {
				chunk
					.tile_positions()
					.map(move |pos| (pos, chunk.tiles[pos.chunk_relative().chunk_index()]))
			})
			.filter(|(_, pair)| !pair.is_empty());
		for (pos, tile) in tiles {
			tile.into_entity(pos, cmd, assets);
		}
	}

	pub fn used_chunks(&self) -> (ChunkPos, ChunkPos) {
		let mut min = ChunkPos::of(i32::MAX, i32::MAX);
		let mut max = ChunkPos::of(i32::MIN, i32::MIN);
		for &pos in self.chunks.keys() {
			*min = min.min(*pos);
			*max = max.max(*pos);
		}
		(min, max)
	}

	pub fn used_tiles(&self) -> (TilePos, TilePos) {
		let (minChunk, maxChunk) = self.used_chunks();
		let (minChunk, maxChunk) = (&self[minChunk], &self[maxChunk]);

		let mut min = TilePos::of(i32::MAX, i32::MAX);
		for pos in minChunk.tile_positions() {
			let tile = minChunk.tiles[pos.chunk_relative().chunk_index()];
			if !tile.is_empty() {
				*min = min.min(*pos);
			}
		}

		let mut max = TilePos::of(i32::MIN, i32::MIN);
		for pos in maxChunk.tile_positions() {
			let tile = maxChunk.tiles[pos.chunk_relative().chunk_index()];
			if !tile.is_empty() {
				*max = max.max(*pos);
			}
		}

		(min, max)
	}

	pub fn copy_from(&mut self, other: &Self, destination: TilePos) {
		let (from, to) = other.used_tiles();
		for y in from.y ..= to.y {
			let dy = y - from.y;
			for x in from.x ..= to.x {
				let dx = x - from.x;
				let otherPos = (x, y);
				let selfPos = (destination.x + dx, destination.y + dy);
				self[selfPos] = other[otherPos];
			}
		}
	}

	pub fn fill(&mut self, tile: Tile, from: TilePos, to: TilePos) {
		let (from, to) = tilepos_rect(from, to);
		for y in from.y ..= to.y {
			for x in from.x ..= to.x {
				self[(x, y)].set(tile);
			}
		}
	}

	pub fn fill_line(&mut self, tile: Tile, from: TilePos, to: TilePos) {
		let (from, to) = tilepos_rect(from, to);
		if from.y == to.y {
			for x in from.x ..= to.x {
				self[(x, from.y)].set(tile);
			}
		} else if from.x == to.x {
			for y in from.y ..= to.y {
				self[(from.x, y)].set(tile);
			}
		} else {
			assert!(false, "cannot fill diagonal lines");
		}
	}

	pub fn fill_border(&mut self, tile: Tile, from: TilePos, to: TilePos) {
		let (from, to) = tilepos_rect(from, to);
		self.fill_line(tile, TilePos::of(from.x, from.y), TilePos::of(to.x, from.y));
		self.fill_line(tile, TilePos::of(from.x, to.y), TilePos::of(to.x, to.y));
		self.fill_line(tile, TilePos::of(from.x, from.y), TilePos::of(from.x, to.y));
		self.fill_line(tile, TilePos::of(to.x, from.y), TilePos::of(to.x, to.y));
	}
}

fn tilepos_rect(l: TilePos, r: TilePos) -> (TilePos, TilePos) {
	(l.min(*r).into(), l.max(*r).into())
}

impl Index<TilePos> for Map {
	type Output = TilePair;

	fn index(&self, index: TilePos) -> &Self::Output {
		let chunk = index.into();
		let chunk = self
			.chunks
			.get(&chunk)
			.expect("Attempting to read from chunk that has not been created");
		let index = index.chunk_relative();
		&chunk.tiles[index.chunk_index()]
	}
}

impl IndexMut<TilePos> for Map {
	fn index_mut(&mut self, index: TilePos) -> &mut Self::Output {
		let pos = index.into();
		let chunk = self.chunks.entry(pos).or_insert_with(|| Chunk::new(pos));
		let index = index.chunk_relative();
		&mut chunk.tiles[index.chunk_index()]
	}
}

impl Index<(i32, i32)> for Map {
	type Output = TilePair;

	fn index(&self, (x, y): (i32, i32)) -> &Self::Output {
		&self[TilePos::of(x, y)]
	}
}

impl IndexMut<(i32, i32)> for Map {
	fn index_mut(&mut self, (x, y): (i32, i32)) -> &mut Self::Output {
		&mut self[TilePos::of(x, y)]
	}
}

impl Index<ChunkPos> for Map {
	type Output = Chunk;

	fn index(&self, pos: ChunkPos) -> &Self::Output {
		self.chunks
			.get(&pos)
			.expect("Attempting to get chunk that has not been created")
	}
}

impl IndexMut<ChunkPos> for Map {
	fn index_mut(&mut self, pos: ChunkPos) -> &mut Self::Output {
		self.chunks
			.get_mut(&pos)
			.expect("Attempting to get chunk that has not been created")
	}
}
