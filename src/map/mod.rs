pub mod data;
pub mod gen;

use std::cell::{RefCell, RefMut, OnceCell};
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt::Debug;
use std::ops::{Deref, DerefMut, Index, IndexMut};
use std::rc::Rc;

use anyhow::anyhow;
use bevy::asset::AssetIo;
use bevy::math::{ivec2, uvec2, vec2};
use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use rand::rngs::SmallRng;
use rand::{thread_rng, Rng, SeedableRng};
use serde::Deserialize;

pub use self::data::*;
use crate::{IsoSprite, IsoSpriteBundle, AResult};

pub const tileDiameter: f32 = 64.0;
pub const tileRadius: f32 = tileDiameter / 2.0;

#[derive(Clone, Copy, Debug, Default, Deserialize)]
pub struct Tile {
	pub ty: TileType,
	#[serde(default)]
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
				texture,
				sprite: IsoSprite {
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

#[derive(Clone, Copy, Debug, Default, Deserialize)]
pub struct TilePair {
	#[serde(default)]
	pub foreground: Tile,
	pub background: Tile,

	/// Whether this tile has been replaced by a dynamic entity.
	#[serde(default)]
	pub plucked: bool,
}

impl TilePair {
	pub fn is_empty(&self) -> bool {
		self.foreground.is_empty() && self.background.is_empty()
	}

	pub fn is_wall(&self) -> bool {
		matches!(self.foreground.ty, TileType::Wall(_))
	}

	pub fn is_floor(&self) -> bool {
		self.foreground.is_empty() && matches!(self.background.ty, TileType::Floor(_))
	}

	pub fn is_door(&self) -> bool {
		matches!(
			self.foreground.ty,
			TileType::DoorNS { .. } | TileType::DoorEW { .. }
		)
	}

	pub fn is_landmark(&self) -> bool {
		matches!(self.foreground.ty, TileType::Landmark { .. })
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
			mut foreground,
			background,
			plucked,
		} = self;
		if plucked {
			// plucked tiles are rendered by dynamic entities
			foreground.ty = TileType::Empty;
		}

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

	pub const fn new(pos: ChunkPos) -> Self {
		let empty = Tile {
			ty: TileType::Empty,
			tileset: Tileset::Normal,
		};
		let empty = TilePair {
			foreground: empty,
			background: empty,
			plucked: false,
		};
		Self {
			pos,
			tiles: [empty; Self::diameterTiles.pow(2)],
		}
	}

	pub fn is_empty(&self) -> bool {
		self.tiles.iter().all(TilePair::is_empty)
	}

	/// Returns iterator of all (absolute) tile positions stored in this chunk.
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
pub struct MapRng(Rc<RefCell<SmallRng>>);

impl MapRng {
	pub fn new(rng: SmallRng) -> Self {
		Self(Rc::new(RefCell::new(rng)))
	}

	pub fn as_mut(&self) -> RefMut<'_, SmallRng> {
		self.0.borrow_mut()
	}
}

#[derive(Clone, Debug, Resource)]
pub struct Map {
	pub chunks: HashMap<ChunkPos, Chunk>,
}

impl Map {
	pub fn new() -> Self {
		Self {
			chunks: HashMap::new(),
		}
	}

	/// Returns minimum/maximum positions of chunks that are nonempty.
	pub fn used_chunks(&self) -> (ChunkPos, ChunkPos) {
		let mut min = ChunkPos::of(i32::MAX, i32::MAX);
		let mut max = ChunkPos::of(i32::MIN, i32::MIN);
		for &pos in self.chunks.keys() {
			if self[pos].is_empty() {
				continue;
			}
			*min = min.min(*pos);
			*max = max.max(*pos);
		}
		(min, max)
	}

	/// Returns minimum/maximum positions of tiles that are nonempty.
	pub fn used_tiles(&self) -> TileRect {
		let (minChunk, maxChunk) = self.used_chunks();
		let borderChunks = (minChunk.x ..= maxChunk.x)
			.map(|x| ChunkPos::of(x, minChunk.y))
			.chain((minChunk.x ..= maxChunk.x).map(|x| ChunkPos::of(x, maxChunk.y)))
			.chain((minChunk.y + 1 ..= maxChunk.y - 1).map(|y| ChunkPos::of(minChunk.x, y)))
			.chain((minChunk.y + 1 ..= maxChunk.y - 1).map(|y| ChunkPos::of(maxChunk.x, y)));

		let mut min = TilePos::of(i32::MAX, i32::MAX);
		let mut max = TilePos::of(i32::MIN, i32::MIN);
		for chunkPos in borderChunks {
			let chunk = &self[chunkPos];
			for (tile, tilePos) in chunk.tiles.iter().zip(chunk.tile_positions()) {
				if !tile.is_empty() {
					*min = min.min(*tilePos);
					*max = max.max(*tilePos);
				}
			}
		}
		TileRect::new_presorted(min, max)
	}

	pub fn find_tile(
		&self,
		position: TilePos,
		mut predicate: impl FnMut(TilePos, &TilePair) -> bool,
	) -> Option<TilePos> {
		let (minChunk, maxChunk) = self.used_chunks();
		let chunksX = (minChunk.x ..= maxChunk.x);
		let chunksY = (minChunk.y ..= maxChunk.y);

		let mut enqueued = HashSet::new();
		let mut queue = VecDeque::new();
		enqueued.insert(position);
		queue.push_back(position);
		while !queue.is_empty() {
			let tile = queue.pop_front().unwrap();

			if predicate(tile, &self[tile]) {
				return Some(tile);
			}

			for other in tile.moore_neighborhood() {
				let chunk = ChunkPos::from(other);
				let outOfBounds = !(chunksX.contains(&chunk.x) && chunksY.contains(&chunk.y));
				if outOfBounds || enqueued.contains(&other) {
					continue;
				}

				enqueued.insert(other);
				queue.push_back(other);
			}
		}
		None
	}
}

impl Index<ChunkPos> for Map {
	type Output = Chunk;

	fn index(&self, pos: ChunkPos) -> &Self::Output {
		static ghostChunk: Chunk = Chunk::new(ChunkPos::of(i32::MAX, i32::MAX));
		self.chunks.get(&pos).unwrap_or(&ghostChunk)
	}
}

impl IndexMut<ChunkPos> for Map {
	fn index_mut(&mut self, pos: ChunkPos) -> &mut Self::Output {
		self.chunks.entry(pos).or_insert_with(|| Chunk::new(pos))
	}
}

impl Index<TilePos> for Map {
	type Output = TilePair;

	fn index(&self, index: TilePos) -> &Self::Output {
		let chunkPos = ChunkPos::from(index);
		let chunk = &self[chunkPos];
		let index = index.chunk_relative();
		&chunk.tiles[index.chunk_index()]
	}
}

impl IndexMut<TilePos> for Map {
	fn index_mut(&mut self, index: TilePos) -> &mut Self::Output {
		let chunkPos = ChunkPos::from(index);
		let chunk = &mut self[chunkPos];
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

#[derive(Clone, Debug)]
pub struct MutMap {
	pub map: Map,
	pub rng: MapRng,
}

impl MutMap {
	pub fn new(seed: Option<u64>) -> Self {
		let seed = seed.unwrap_or_else(|| thread_rng().gen());
		Self {
			rng: MapRng::new(SmallRng::seed_from_u64(seed)),
			map: Map::new(),
		}
	}

	pub fn from_rng(rng: &MapRng) -> Self {
		Self {
			rng: rng.clone(),
			map: Map::new(),
		}
	}

	pub fn into_entities(self, cmd: &mut Commands, assets: &AssetServer) -> Map {
		let tiles = self
			.chunks
			.iter()
			.flat_map(|(pos, chunk)| {
				chunk
					.tile_positions()
					.map(move |pos| (pos, chunk.tiles[pos.chunk_relative().chunk_index()]))
			})
			.filter(|(_, pair)| !pair.is_empty());
		for (pos, tile) in tiles {
			tile.into_entity(pos, cmd, assets);
		}

		self.map
	}

	pub fn rng(&self) -> RefMut<'_, SmallRng> {
		self.rng.as_mut()
	}

	/// Copies all of `other` into `self`, with `other`'s min [`used_tiles`]
	/// placed at `destination`.
	pub fn copy_from(&mut self, other: &Self, destination: TilePos) {
		let rect = other.used_tiles();
		for y in rect.min.y ..= rect.max.y {
			let dy = y - rect.min.y;
			for x in rect.min.x ..= rect.max.x {
				let dx = x - rect.min.x;
				let otherPos = (x, y);
				let selfPos = (destination.x + dx, destination.y + dy);
				self[selfPos] = other[otherPos];
			}
		}
	}

	/// Sets all tiles in a rect spanning `from ..= to`.
	pub fn fill(&mut self, tile: Tile, from: TilePos, to: TilePos) {
		let rect = TileRect::new(from, to);
		for y in rect.min.y ..= rect.max.y {
			for x in rect.min.x ..= rect.max.x {
				self[(x, y)].set(tile);
			}
		}
	}

	/// Sets all tiles in a line. Only axis-aligned lines are supported.
	pub fn fill_line(&mut self, tile: Tile, from: TilePos, to: TilePos) {
		let rect = TileRect::new(from, to);
		if rect.min.y == rect.max.y {
			for x in rect.min.x ..= rect.max.x {
				self[(x, rect.min.y)].set(tile);
			}
		} else if rect.min.x == rect.max.x {
			for y in rect.min.y ..= rect.max.y {
				self[(rect.min.x, y)].set(tile);
			}
		} else {
			assert!(false, "cannot fill diagonal lines");
		}
	}

	/// Sets all tiles on the border of a rect spanning `from ..= to`.
	pub fn fill_border(&mut self, tile: Tile, from: TilePos, to: TilePos) {
		let rect = TileRect::new(from, to);
		self.fill_line(
			tile,
			TilePos::of(rect.min.x, rect.min.y),
			TilePos::of(rect.max.x, rect.min.y),
		);
		self.fill_line(
			tile,
			TilePos::of(rect.min.x, rect.max.y),
			TilePos::of(rect.max.x, rect.max.y),
		);
		self.fill_line(
			tile,
			TilePos::of(rect.min.x, rect.min.y),
			TilePos::of(rect.min.x, rect.max.y),
		);
		self.fill_line(
			tile,
			TilePos::of(rect.max.x, rect.min.y),
			TilePos::of(rect.max.x, rect.max.y),
		);
	}

	/// Returns all tiles matching the given `predicate`, and marks them as
	/// having been [plucked](`TilePair::plucked`).
	pub fn pluck_tiles(
		&mut self,
		mut predicate: impl FnMut(TilePos, &TilePair) -> bool,
	) -> Vec<(TilePos, Tile)> {
		let mut res = vec![];
		for pos in self.used_tiles().tiles() {
			let tile = self[pos];
			if tile.plucked || // don't double-pluck tiles
				!predicate(pos, &tile)
			{
				continue;
			}

			self[pos].plucked = true;
			res.push((pos, tile.foreground));
		}
		res
	}
}

impl Deref for MutMap {
	type Target = Map;

	fn deref(&self) -> &Self::Target {
		&self.map
	}
}

impl DerefMut for MutMap {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.map
	}
}

#[derive(Clone, Debug, Deserialize)]
pub struct Prefab {
	key: HashMap<char, TilePair>,
	map: Vec<Box<str>>,
	#[serde(skip)]
	size: OnceCell<UVec2>,
}

impl Prefab {
	pub fn load_blocking(assets: &AssetServer, path: &str) -> AResult<Self> {
		let io = assets
			.asset_io()
			.downcast_ref::<bevy::asset::FileAssetIo>()
			.ok_or_else(|| anyhow!("wef"))?;
		let root = io.root_path();
		let path = root.join(path);

		let str = std::fs::read_to_string(path)?;
		Ok(ron::from_str(&str)?)
	}

	pub fn into_map(self, seed: Option<u64>) -> AResult<MutMap> {
		let mut res = MutMap::new(seed);
		for (pos, tile) in self.iter() {
			res[pos] = tile;
		}
		Ok(res)
	}

	pub fn size(&self) -> UVec2 {
		*self.size.get_or_init(|| UVec2::new(
			self.map.iter().map(|s| s.len()).max().unwrap_or(0) as _,
			self.map.len() as _,
		))
	}

	pub fn iter(&self) -> impl '_ + Iterator<Item = (TilePos, TilePair)> {
		self.map.iter().enumerate().flat_map(move |(y, line)|
			line.chars().enumerate().filter_map(move |(x, char)|
				if char == ' ' { None } else { Some((TilePos::of(x as _, y as _), self.key[&char])) }
			)
		)
	}

	pub fn copy_into(&self, map: &mut MutMap, origin: TilePos) {
		for (pos, tile) in self.iter() {
			let pos = TilePos::from(*pos + *origin);
			map[pos] = tile;
		}
	}
}
