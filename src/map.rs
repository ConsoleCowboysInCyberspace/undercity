use std::collections::HashMap;
use std::fmt::Debug;
use std::ops::{Deref, DerefMut, Index, IndexMut};

use bevy::math::{ivec2, uvec2, vec2};
use bevy::prelude::*;
use rand::{thread_rng, Rng};

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
#[repr(u8)]
pub enum Tileset {
	#[default]
	BrickBlue,
	BrickCyan,
	BrickGreen,
	BrickPurple,
	BrickRed,
	BrickYellow,
	Catacomb,
	Cocutos,
	Crypt,
	Gallery,
	Gehena,
	Hive,
	Lair,
	Lapis,
	Moss,
	Mucus,
	Normal,
	PandemBlue,
	PandemGreen,
	PandemPurple,
	PandemRed,
	PandemYellow,
	Rock,
	Tunnel,
}

impl Tileset {
	pub fn asset_path(self) -> &'static str {
		match self {
			Self::BrickBlue => "tiles/brick_blue.png",
			Self::BrickCyan => "tiles/brick_cyan.png",
			Self::BrickGreen => "tiles/brick_green.png",
			Self::BrickPurple => "tiles/brick_purple.png",
			Self::BrickRed => "tiles/brick_red.png",
			Self::BrickYellow => "tiles/brick_yellow.png",
			Self::Catacomb => "tiles/catacomb.png",
			Self::Cocutos => "tiles/cocutos.png",
			Self::Crypt => "tiles/crypt.png",
			Self::Gallery => "tiles/gallery.png",
			Self::Gehena => "tiles/gehena.png",
			Self::Hive => "tiles/hive.png",
			Self::Lair => "tiles/lair.png",
			Self::Lapis => "tiles/lapis.png",
			Self::Moss => "tiles/moss.png",
			Self::Mucus => "tiles/mucus.png",
			Self::Normal => "tiles/normal.png",
			Self::PandemBlue => "tiles/pandem_blue.png",
			Self::PandemGreen => "tiles/pandem_green.png",
			Self::PandemPurple => "tiles/pandem_purple.png",
			Self::PandemRed => "tiles/pandem_red.png",
			Self::PandemYellow => "tiles/pandem_yellow.png",
			Self::Rock => "tiles/rock.png",
			Self::Tunnel => "tiles/tunnel.png",
		}
	}
}

#[derive(Clone, Copy, Debug, Default)]
#[repr(u8)] // tileset atlases
pub enum WallShape {
	#[default]
	Pillar = 0,

	North = 1,
	East = 2,
	South = 8,
	West = 4,

	Northeast = 3,
	Northwest = 5,
	Southeast = 10,
	Southwest = 12,

	Eastwest = 6,
	Northsouth = 9,

	Solid = 15,
	SolidNorth = 7,
	SolidEast = 11,
	SolidSouth = 14,
	SolidWest = 13,
}

#[derive(Clone, Copy, Debug, Default)]
#[repr(u8)] // misc atlas
pub enum Landmark {
	#[default]
	Well = 67,

	StatueDragon = 68,
	StatueFace = 69,
	StatueBronze = 70,

	StairsMarbleTop = 81,
	StairsMarbleBottom = 85,
	StairsSandstoneTop = 89,
	StairsSandstoneBottom = 90,

	TrapArrow = 78,
	TrapPentagram = 79,
	TrapSkull = 80,

	PortalLight = 91,
	PortalDark = 92,
	PortalRed = 93,
	PortalBlue = 94,
	PortalGreen = 95,
	PortalSkulls = 96,
	PortalStar = 97,
	PortalArch = 98,
	PortalDemon = 99,
	PortalWormhole = 101,
	PortalBlank = 102,

	ShrinePalm = 103,
	ShrineIdol = 104,
	ShrineSkulls = 105,
	ShrineGeode = 106,
	ShrineFace = 107,
	ShrineScroll = 108,
	ShrineCross = 109,
	ShrineFlame = 110,
	ShrineLapis = 111,
	ShrineSacrifice = 112,
	ShrineDemon = 113,
	ShrineUrn = 114,
	ShrineChair = 115,
}

#[derive(Clone, Copy, Debug, Default)]
#[repr(u8)] // misc atlas (except `::Tileset`)
pub enum FloorType {
	#[default]
	Tileset = 20,

	Black = 0,
	LavaRed = 71,
	LavaBlue = 72,
	LavaCyan = 73,
	Slab = 74,
}

#[derive(Clone, Copy, Debug, Default)]
pub enum TileType {
	#[default]
	Empty,
	Floor(FloorType),
	Wall(WallShape),
	DoorNS {
		open: bool,
	},
	DoorEW {
		open: bool,
	},
	Landmark {
		ty: Landmark,
		flip: bool,
	},
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Tile {
	pub ty: TileType,
	pub tileset: Tileset,
}

impl Tile {
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

#[derive(Clone)]
pub struct Chunk {
	pub tiles: [Tile; Self::diameterTiles.pow(2)],
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

	pub fn into_tiles(self) -> impl Iterator<Item = (TilePos, Tile)> {
		self.chunks
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
			.filter(|(_, tile)| !matches!(tile.ty, TileType::Empty))
	}
}

impl Index<TilePos> for Map {
	type Output = Tile;

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
	type Output = Tile;

	fn index(&self, (x, y): (i32, i32)) -> &Self::Output {
		&self[TilePos::of(x, y)]
	}
}

impl IndexMut<(i32, i32)> for Map {
	fn index_mut(&mut self, (x, y): (i32, i32)) -> &mut Self::Output {
		&mut self[TilePos::of(x, y)]
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TilePos(IVec2);

impl TilePos {
	pub fn of(x: i32, y: i32) -> Self {
		Self(ivec2(x, y))
	}

	fn chunk_relative(self) -> Self {
		Self(ivec2(self.x & 0x1F, self.y & 0x1F))
	}
}

impl Deref for TilePos {
	type Target = IVec2;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl DerefMut for TilePos {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}

impl From<IVec2> for TilePos {
	fn from(vec: IVec2) -> Self {
		Self(vec)
	}
}

impl From<TilePos> for IVec2 {
	fn from(this: TilePos) -> Self {
		this.0
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ChunkPos(IVec2);

impl ChunkPos {
	pub fn of(x: i32, y: i32) -> Self {
		Self(ivec2(x, y))
	}
}

impl Deref for ChunkPos {
	type Target = IVec2;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl DerefMut for ChunkPos {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}

impl From<IVec2> for ChunkPos {
	fn from(vec: IVec2) -> Self {
		Self(vec)
	}
}

impl From<ChunkPos> for IVec2 {
	fn from(this: ChunkPos) -> Self {
		this.0
	}
}

impl From<TilePos> for ChunkPos {
	fn from(pos: TilePos) -> Self {
		Self::of(pos.x >> 5, pos.y >> 5)
	}
}
