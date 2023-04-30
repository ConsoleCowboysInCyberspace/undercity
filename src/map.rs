use bevy::math::{uvec2, vec2};
use bevy::prelude::*;

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
#[repr(u8)] // misc atlas
pub enum FloorType {
	#[default]
	Tileset = 255,

	Black = 0,
	LavaRed = 71,
	LavaBlue = 72,
	LavaCyan = 73,
	Slab = 74,
}

impl FloorType {
	fn atlas_index(self) -> usize {
		match self {
			Self::Tileset => 20, // index in tileset
			_ => self as _,
		}
	}
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
	Landmark(Landmark),
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Tile {
	pub tileset: Tileset,
	pub tile: TileType,
}

impl Tile {
	pub fn texture_info(&self) -> (&'static str, Rect) {
		let (index, texture) = match self.tile {
			TileType::Empty => unimplemented!("Should never convert empty tiles into tile bundle"),
			TileType::Floor(floor) => (
				floor.atlas_index(),
				(!matches!(floor, FloorType::Tileset)).then_some("tiles/misc.png"),
			),
			TileType::Wall(shape) => (shape as _, None),
			TileType::DoorNS { open } => (17 + open.then_some(2).unwrap_or(0), None),
			TileType::DoorEW { open } => (16 + open.then_some(2).unwrap_or(0), None),
			TileType::Landmark(landmark) => (landmark as _, Some("tiles/misc.png")),
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
		)
	}

	pub fn into_bundle(self, pos: Vec2, assets: &AssetServer) -> TileBundle {
		let (texture, rect) = self.texture_info();
		let texture = assets.load(texture);
		let rect = Some(rect);

		TileBundle {
			isoTransform: IsoTransform {
				pos,
				scale: tileRadius,
			},
			sprite: SpriteBundle {
				texture,
				sprite: Sprite { rect, ..default() },
				..default()
			},
		}
	}
}

pub struct Map {
	pub size: UVec2,
	pub tiles: Vec<Tile>,
}

impl Map {
	pub fn new(size: UVec2) -> Self {
		Self {
			size,
			tiles: vec![Tile::default(); (size.x * size.y) as _],
		}
	}

	pub fn into_tiles(self) -> impl Iterator<Item = (Vec2, Tile)> {
		self.tiles
			.into_iter()
			.enumerate()
			.filter(|(_, tile)| !matches!(tile.tile, TileType::Empty))
			.map(move |(index, tile)| {
				let pos = vec2(
					(index as u32 % self.size.x) as _,
					(index as u32 / self.size.x) as _,
				);
				(pos, tile)
			})
	}
}
