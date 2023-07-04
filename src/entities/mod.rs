pub mod door;
pub mod player;
pub mod shrine;

use bevy::prelude::*;

#[derive(Clone, Component, Debug)]
pub struct Health(f32);

impl Health {
	pub fn new(initial: f32) -> Self {
		Self(initial)
	}

	pub fn is_dead(&self) -> bool {
		self.0 <= 0.0
	}

	pub fn take_damage(&mut self, amount: f32) {
		self.0 -= amount;
		dbg!(self);
	}

	pub fn take_healing(&mut self, amount: f32) {
		self.0 += amount;
		dbg!(self);
	}
}
