use core::panic;
use std::collections::VecDeque;

use rand::rngs::SmallRng;
use rand::seq::SliceRandom;
use rand::SeedableRng;

use super::data::Direction;
use super::*;

impl TileRect {
	pub fn tiles(self) -> impl Iterator<Item = TilePos> {
		(self.min.y ..= self.max.y)
			.flat_map(move |y| (self.min.x ..= self.max.x).map(move |x| TilePos::of(x, y)))
	}

	pub fn split(&self, xAxis: bool, firstWidth: i32) -> (Self, Self) {
		if xAxis {
			let mid = self.min.x + firstWidth;
			(
				Self::new_presorted(self.min, TilePos::of(mid, self.max.y)),
				Self::new_presorted(TilePos::of(mid, self.min.y), self.max),
			)
		} else {
			let mid = self.min.y + firstWidth;
			(
				Self::new_presorted(self.min, TilePos::of(self.max.x, mid)),
				Self::new_presorted(TilePos::of(self.min.x, mid), self.max),
			)
		}
	}

	pub fn tile_on_border(&self, rng: &mut impl Rng) -> TilePos {
		let dir: u32 = rng.gen_range(0 .. 4);
		match dir {
			0 | 1 => {
				let x = rng.gen_range(self.min.x ..= self.max.x);
				let y = if dir & 1 == 0 { self.min.y } else { self.max.y };
				TilePos::of(x, y)
			},
			2 | 3 => {
				let x = if dir & 1 == 0 { self.min.x } else { self.max.x };
				let y = rng.gen_range(self.min.y ..= self.max.y);
				TilePos::of(x, y)
			},
			_ => unreachable!(),
		}
	}
}

pub fn generate_map(seed: u64) -> (Map, TilePos) {
	let mut rng = SmallRng::seed_from_u64(seed);

	let mut allRects = vec![];
	let mut roomRects = vec![];
	let mut queue = VecDeque::new();
	let mapRect = TileRect::new(TilePos::of(-60, -60), TilePos::of(60, 60));
	queue.push_front((0, false, mapRect));
	while let Some((depth, xAxis, rect)) = queue.pop_back() {
		allRects.push(rect);
		if depth > 3 {
			roomRects.push(rect);
			continue;
		}

		let size = rect.size();
		let size = if xAxis { size.x } else { size.y };
		let size = size / 2 + rng.gen_range(-size / 4 .. size / 4);
		let (l, r) = rect.split(xAxis, size);
		queue.push_back((depth + 1, !xAxis, l));
		queue.push_back((depth + 1, !xAxis, r));
	}

	let mut res = Map::new();

	// fill hallway floors
	for rect in allRects {
		res.fill_border(
			Tile {
				ty: TileType::Floor(FloorType::Tileset),
				tileset: Tileset::Rock,
			},
			rect.min,
			rect.max,
		);
	}

	// generate rooms
	let mut doors = vec![];
	for mut rect in roomRects.iter().copied() {
		let borderSize = rng.gen_range(3 .. 7);
		*rect.min += IVec2::splat(borderSize);
		*rect.max -= IVec2::splat(borderSize);

		let mut trect = rect;
		trect.translate(-*rect.min);
		let (room, roomDoors) = generate_room(&mut rng, trect);
		res.copy_from(&room, rect.min);
		doors.extend(roomDoors.into_iter().map(|p| TilePos::from(*p + *rect.min)));
	}

	// pave paths from doors to hallways
	'paving: for pos in doors {
		assert!(res[pos].is_door());
		let dir = match res[pos].foreground.ty {
			TileType::DoorNS { .. } => {
				if res[pos.neighbor(Direction::East)].is_floor() {
					Direction::West
				} else {
					Direction::East
				}
			},
			TileType::DoorEW { .. } => {
				if res[pos.neighbor(Direction::North)].is_floor() {
					Direction::South
				} else {
					Direction::North
				}
			},
			_ => unreachable!(),
		};

		for tiles in 1 .. 250 {
			let newPos = TilePos::from(*pos + dir.delta() * tiles);
			if !res[newPos].is_empty() {
				continue 'paving;
			}
			res[newPos].set(Tile {
				ty: TileType::Floor(FloorType::Tileset),
				tileset: Tileset::Rock,
			});
		}
		eprintln!("couldn't connect door at {pos:?} to hallways");
	}

	// place walls around hallways
	let mapRect = res.used_tiles();
	for pos in mapRect.tiles() {
		if !res[pos].is_floor() {
			continue;
		}
		for neighbor in pos.moore_neighborhood() {
			if res[neighbor].is_empty() {
				res[neighbor].set(Tile {
					ty: TileType::Wall(WallShape::Solid),
					tileset: Tileset::Rock,
				});
			}
		}
	}

	// pick player spawnpoint
	let mut iters = 0;
	let playerSpawn = loop {
		if iters > 1000 {
			panic!("couldn't find anywhere to spawn player");
		}
		iters += 1;

		let room = roomRects.choose(&mut rng).unwrap();
		let pos = TilePos::of(
			rng.gen_range(room.min.x ..= room.max.x),
			rng.gen_range(room.min.y ..= room.max.y),
		);

		if !res[pos].is_floor() {
			continue;
		}
		// FIXME: place several of these and pick one in `into_entities` or something
		// res[pos].foreground.ty = TileType::Landmark { ty: Landmark::SpawnPlayer,
		// flip: false };
		break pos;
	};

	(res, playerSpawn)
}

fn generate_room(rng: &mut SmallRng, rect: TileRect) -> (Map, Vec<TilePos>) {
	let tilesets = [
		Tileset::BrickBlue,
		Tileset::BrickCyan,
		Tileset::BrickGreen,
		Tileset::BrickPurple,
		Tileset::BrickRed,
		Tileset::BrickYellow,
		Tileset::Catacomb,
		Tileset::Cocutos,
		Tileset::Crypt,
		Tileset::Gallery,
		Tileset::Gehena,
		Tileset::Hive,
		Tileset::Lair,
		Tileset::Lapis,
		Tileset::Moss,
		Tileset::Mucus,
		Tileset::Normal,
		Tileset::PandemBlue,
		Tileset::PandemGreen,
		Tileset::PandemPurple,
		Tileset::PandemRed,
		Tileset::PandemYellow,
		Tileset::Rock,
		Tileset::Tunnel,
	];

	let mut res = Map::new();
	let tileset = *tilesets.choose(rng).unwrap();
	res.fill(
		Tile {
			ty: TileType::Floor(FloorType::Tileset),
			tileset,
		},
		rect.min,
		rect.max,
	);
	res.fill_border(
		Tile {
			ty: TileType::Wall(WallShape::Solid),
			tileset,
		},
		rect.min,
		rect.max,
	);

	// place doors
	let mut doors = vec![];
	for _ in 0 .. rng.gen_range(1 ..= 4) {
		'placing: for _ in 0 .. 1000 {
			let pos = rect.tile_on_border(rng);
			for neighbor in pos.von_neumann_neighborhood() {
				if res[neighbor].is_door() {
					continue 'placing;
				}
			}

			let door = if res[pos.neighbor(Direction::North)].is_wall() &&
				res[pos.neighbor(data::Direction::South)].is_wall()
			{
				TileType::DoorNS { open: false }
			} else if res[pos.neighbor(Direction::East)].is_wall() &&
				res[pos.neighbor(data::Direction::West)].is_wall()
			{
				TileType::DoorEW { open: false }
			} else {
				// trying to place door on a corner?
				continue 'placing;
			};
			res[pos].set(Tile { ty: door, tileset });
			doors.push(pos);
			break;
		}
		// TODO: reroll room? maybe wrap in a `Result` for other fallible generation
		assert!(!doors.is_empty(), "could not place any doors");
	}

	(res, doors)
}
