pub mod data;

use std::collections::HashMap;
use std::fmt::Debug;
use std::ops::{Deref, DerefMut, Index, IndexMut};

use bevy::math::{ivec2, uvec2, vec2};
use bevy::prelude::*;
use rand::{thread_rng, Rng};

pub use self::data::*;
use crate::IsoTransform;

pub const tileDiameter: f32 = 64.0;
pub const tileRadius: f32 = tileDiameter / 2.0;

#[derive(Clone, Bundle)]
pub struct TileBundle {
	pub isoTransform: IsoTransform,

	#[bundle]
	pub sprite: SpriteBundle,
}

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

	pub fn into_bundle(self, pos: Vec2, assets: &AssetServer) -> TileBundle {
		let (texture, rect, flip) = self.texture_info();
		let texture = assets.load(texture);
		let rect = Some(rect);

		TileBundle {
			isoTransform: IsoTransform {
				pos,
				scale: tileRadius,
			},
			sprite: SpriteBundle {
				texture,
				sprite: Sprite {
					rect,
					flip_x: flip,
					..default()
				},
				..default()
			},
		}
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
			cmd.spawn(IsoTransform {
				pos,
				scale: tileRadius,
			})
		} else {
			cmd.spawn(foreground.into_bundle(pos, assets))
		};

		if !background.is_empty() {
			let background = background.into_bundle(pos, assets);
			foreground.with_children(|b| {
				b.spawn(SpriteBundle {
					transform: Transform::from_xyz(0.0, 0.0, -0.5),
					..background.sprite
				});
			});
		}

		foreground.id()
	}
}

#[derive(Clone)]
pub struct Chunk {
	pub tiles: [TilePair; Self::diameterTiles.pow(2)],
}

impl Chunk {
	pub const diameterTiles: usize = 32;
}

impl Default for Chunk {
	fn default() -> Self {
		Self {
			tiles: [default(); Self::diameterTiles.pow(2)],
		}
	}
}

impl Debug for Chunk {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Chunk")
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
				(0 .. Chunk::diameterTiles as i32).flat_map(move |y| {
					(0 .. Chunk::diameterTiles as i32).map(move |x| {
						let tilePos = TilePos::of(pos.x << 5 | x, pos.y << 5 | y);
						let tile = chunk.tiles[(y * Chunk::diameterTiles as i32 + x) as usize];
						(tilePos, tile)
					})
				})
			})
			.filter(|(_, pair)| !pair.is_empty());
		for (pos, tile) in tiles {
			tile.into_entity(pos, cmd, assets);
		}
	}
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
		&chunk.tiles[(index.y * Chunk::diameterTiles as i32 + index.x) as usize]
	}
}

impl IndexMut<TilePos> for Map {
	fn index_mut(&mut self, index: TilePos) -> &mut Self::Output {
		let chunk = index.into();
		let chunk = self.chunks.entry(chunk).or_default();
		let index = index.chunk_relative();
		&mut chunk.tiles[(index.y * Chunk::diameterTiles as i32 + index.x) as usize]
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
