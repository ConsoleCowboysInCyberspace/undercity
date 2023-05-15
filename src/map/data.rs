use std::ops::{Deref, DerefMut};

use bevy::ecs::system::EntityCommands;
use bevy::math::{ivec2, vec2};
use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

use super::tileRadius;
use crate::map::Chunk;

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

#[derive(Clone, Debug, Default)]
pub struct PositionedCollider {
	pub collider: Collider,
	pub translation: Option<Vec2>,
}

impl PositionedCollider {
	pub fn insert_into(self, ent: &mut EntityCommands) {
		let debugRender = ColliderDebugColor(Color::ORANGE_RED);
		if let Some(translation) = self.translation {
			ent.with_children(|b| {
				b.spawn((
					self.collider,
					debugRender,
					TransformBundle::from(Transform::from_translation((translation, 0.0).into())),
				));
			});
		} else {
			ent.insert((self.collider, debugRender));
		}
	}
}

impl From<Collider> for PositionedCollider {
	fn from(collider: Collider) -> Self {
		Self {
			collider,
			translation: None,
		}
	}
}

impl From<(Collider, Vec2)> for PositionedCollider {
	fn from((collider, position): (Collider, Vec2)) -> Self {
		let position = Some(position);
		Self {
			collider,
			translation: position,
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

impl WallShape {
	pub fn collider(self) -> PositionedCollider {
		const fullRadius: f32 = tileRadius / 2.0;
		const pillarRadius: f32 = fullRadius * 0.55;
		const divotSize: f32 = (fullRadius - pillarRadius) / 2.0;
		match self {
			WallShape::Pillar => Collider::cuboid(pillarRadius, pillarRadius).into(),
			WallShape::North => (
				Collider::cuboid(pillarRadius, pillarRadius + divotSize),
				vec2(0.0, -divotSize),
			)
				.into(),
			WallShape::East => (
				Collider::cuboid(pillarRadius + divotSize, pillarRadius),
				vec2(divotSize, 0.0),
			)
				.into(),
			WallShape::South => (
				Collider::cuboid(pillarRadius, pillarRadius + divotSize),
				vec2(0.0, divotSize),
			)
				.into(),
			WallShape::West => (
				Collider::cuboid(pillarRadius + divotSize, pillarRadius),
				vec2(-divotSize, 0.0),
			)
				.into(),
			WallShape::Northeast => (
				Collider::cuboid(pillarRadius + divotSize, pillarRadius + divotSize),
				vec2(divotSize, -divotSize),
			)
				.into(),
			WallShape::Northwest => (
				Collider::cuboid(pillarRadius + divotSize, pillarRadius + divotSize),
				vec2(-divotSize, -divotSize),
			)
				.into(),
			WallShape::Southeast => (
				Collider::cuboid(pillarRadius + divotSize, pillarRadius + divotSize),
				vec2(divotSize, divotSize),
			)
				.into(),
			WallShape::Southwest => (
				Collider::cuboid(pillarRadius + divotSize, pillarRadius + divotSize),
				vec2(-divotSize, divotSize),
			)
				.into(),
			WallShape::Eastwest => Collider::cuboid(fullRadius, pillarRadius).into(),
			WallShape::Northsouth => Collider::cuboid(pillarRadius, fullRadius).into(),
			WallShape::Solid => Collider::cuboid(fullRadius, fullRadius).into(),
			WallShape::SolidNorth => (
				Collider::cuboid(fullRadius, pillarRadius + divotSize),
				vec2(0.0, -divotSize),
			)
				.into(),
			WallShape::SolidEast => (
				Collider::cuboid(pillarRadius + divotSize, fullRadius),
				vec2(divotSize, 0.0),
			)
				.into(),
			WallShape::SolidSouth => (
				Collider::cuboid(fullRadius, pillarRadius + divotSize),
				vec2(0.0, divotSize),
			)
				.into(),
			WallShape::SolidWest => (
				Collider::cuboid(pillarRadius + divotSize, fullRadius),
				vec2(-divotSize, 0.0),
			)
				.into(),
		}
	}
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

	SpawnPlayer = 116,
	SpawnWitch = 119,
	SpawnWitchette = 120,
	SpawnJester = 124,
	SpawnRedDemon = 125,
	SpawnYellowDemon = 127,
	SpawnGreenDemon = 130,
	SpawnBlueDemon = 131,
	SpawnWingedDemon = 132,

	// these are just to get the sprite lmao
	Cursor = 165,
	ExplosionRed = 168,
	ExplosionBlue = 171,
	ExplosionGreen = 174,
	ExplosionSmokeLight = 179,
	ExplosionSmokeDark = 180,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TilePos(IVec2);

impl TilePos {
	pub fn of(x: i32, y: i32) -> Self {
		Self(ivec2(x, y))
	}

	/// Returns this tile position relative to its chunk (i.e. in `0
	/// ..`[`Chunk::diameterTiles`].)
	pub fn chunk_relative(self) -> Self {
		Self(ivec2(self.x & 0x1F, self.y & 0x1F))
	}

	/// Returns the index of this tile position in the [`Chunk::tiles`] array.
	pub fn chunk_index(self) -> usize {
		debug_assert!(
			self.x >= 0 &&
				self.x < Chunk::diameterTiles as _ &&
				self.y >= 0 && self.y < Chunk::diameterTiles as _,
			"can only get chunk index of chunk-relative TilePos"
		);
		(self.y * Chunk::diameterTiles as i32 + self.x) as _
	}

	/// Returns neighboring tile in the given direction.
	pub fn neighbor(&self, dir: Direction) -> Self {
		(self.0 + dir.delta()).into()
	}

	/// Returns iterator over all [Moore](https://en.wikipedia.org/wiki/Moore_neighborhood) neighbors (includes corners.)
	pub fn moore_neighborhood(self) -> impl Iterator<Item = Self> {
		use self::Direction::*;
		[North, NorthEast, East, SouthEast, South, SouthWest, West, NorthWest].into_iter()
		.map(move |dir| self.neighbor(dir))
	}

	/// Returns iterator over all [von Neumann](https://en.wikipedia.org/wiki/Von_Neumann_neighborhood) neighbors (excludes corners.)
	pub fn von_neumann_neighborhood(self) -> impl Iterator<Item = Self> {
		use self::Direction::*;
		[North, East, South, West].into_iter()
		.map(move |dir| self.neighbor(dir))
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
	pub const fn of(x: i32, y: i32) -> Self {
		Self(ivec2(x, y))
	}

	/// Returns the northwesternmost tile in this chunk.
	pub fn min_tile(&self) -> TilePos {
		TilePos::of(
			self.x * Chunk::diameterTiles as i32,
			self.y * Chunk::diameterTiles as i32,
		)
	}

	/// Returns the southeasternmost tile in this chunk.
	pub fn max_tile(&self) -> TilePos {
		(*self.min_tile() + IVec2::splat(Chunk::diameterTiles as i32 - 1)).into()
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
	North,
	NorthEast,
	East,
	SouthEast,
	South,
	SouthWest,
	West,
	NorthWest,
}

impl Direction {
	pub fn delta(self) -> IVec2 {
		match self {
			Direction::North => ivec2(0, -1),
			Direction::NorthEast => ivec2(1, -1),
			Direction::East => ivec2(1, 0),
			Direction::SouthEast => ivec2(1, 1),
			Direction::South => ivec2(0, 1),
			Direction::SouthWest => ivec2(-1, 1),
			Direction::West => ivec2(-1, 0),
			Direction::NorthWest => ivec2(-1, -1),
		}
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TileRect {
	pub min: TilePos,
	pub max: TilePos,
}

impl TileRect {
	pub fn new(a: TilePos, b: TilePos) -> Self {
		Self {
			min: a.min(*b).into(),
			max: a.max(*b).into(),
		}
	}

	pub fn new_presorted(min: TilePos, max: TilePos) -> Self {
		Self { min, max }
	}

	pub fn from_origin_size(origin: TilePos, size: IVec2) -> Self {
		Self::new(origin, (*origin + size).into())
	}

	pub fn size(&self) -> IVec2 {
		*self.max - *self.min
	}

	pub fn center(&self) -> TilePos {
		(*self.min + self.size() / 2).into()
	}

	pub fn translate(&mut self, by: IVec2) {
		*self.min += by;
		*self.max += by;
	}

	pub fn intersects(&self, other: &Self) -> bool {
		self.min.x <= other.max.x && other.min.x <= self.max.x &&
		self.min.y <= other.max.y && other.min.y <= self.max.y
	}

	pub fn intersection(&self, other: &Self) -> Option<Self> {
		if !self.intersects(other) {
			return None;
		}

		let min = self.min.max(*other.min).into();
		let max = self.max.min(*other.max).into();
		Some(Self::new_presorted(min, max))
	}
}

impl From<(TilePos, TilePos)> for TileRect {
    fn from((a, b): (TilePos, TilePos)) -> Self {
        Self::new(a, b)
    }
}
