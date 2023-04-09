mod utils;

use fixedbitset::FixedBitSet;
use js_sys::Math;
use std::fmt;
use wasm_bindgen::prelude::*;

extern crate web_sys;
use web_sys::console;

pub struct Timer<'a> {
    name: &'a str,
}

impl<'a> Timer<'a> {
    pub fn new(name: &'a str) -> Timer<'a> {
        console::time_with_label(name);
        Timer { name }
    }
}

impl<'a> Drop for Timer<'a> {
    fn drop(&mut self) {
        console::time_end_with_label(self.name);
    }
}

#[allow(unused_macros)]
macro_rules! log {
    ( $( $t:tt )* ) => {
        console::log_1(&format!( $( $t )* ).into());
    }
}

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cell {
    Dead = 0,
    Alive = 1,
}

#[wasm_bindgen]
pub struct Universe {
    width: u32,
    height: u32,
    cells: FixedBitSet,
    debug: bool,
}

impl Universe {
    fn get_index(&self, row: u32, column: u32) -> usize {
        (row * self.width + column) as usize
    }

    fn get_pos(&self, index: usize) -> (u32, u32) {
        let row = index as u32 / self.width;
        let col = index as u32 % self.width;
        (row, col)
    }

    fn live_neighbor_count(&self, row: u32, column: u32) -> u8 {
        let mut count = 0;

        let north = if row == 0 { self.height - 1 } else { row - 1 };

        let south = if row == self.height - 1 { 0 } else { row + 1 };

        let west = if column == 0 {
            self.width - 1
        } else {
            column - 1
        };

        let east = if column == self.width - 1 {
            0
        } else {
            column + 1
        };

        let nw = self.get_index(north, west);
        count += self.cells[nw] as u8;

        let n = self.get_index(north, column);
        count += self.cells[n] as u8;

        let ne = self.get_index(north, east);
        count += self.cells[ne] as u8;

        let w = self.get_index(row, west);
        count += self.cells[w] as u8;

        let e = self.get_index(row, east);
        count += self.cells[e] as u8;

        let sw = self.get_index(south, west);
        count += self.cells[sw] as u8;

        let s = self.get_index(south, column);
        count += self.cells[s] as u8;

        let se = self.get_index(south, east);
        count += self.cells[se] as u8;

        count
    }

    pub fn get_cells(&self) -> &FixedBitSet {
        &self.cells
    }

    pub fn set_cells(&mut self, cells: &[(u32, u32)]) {
        for (row, col) in cells.iter().cloned() {
            let idx = self.get_index(row, col);
            self.cells.set(idx, true)
        }
    }
}

#[wasm_bindgen]
impl Universe {
    pub fn new() -> Universe {
        utils::set_panic_hook();

        let width = 64;
        let height = 64;

        let size = (width * height) as usize;
        let mut cells = FixedBitSet::with_capacity(size);

        for i in 0..size {
            cells.set(i, i % 2 == 0 || i % 7 == 0);
        }

        Universe {
            width,
            height,
            cells,
            debug: false,
        }
    }

    pub fn new_spaceship() -> Universe {
        let width = 64 as u32;
        let height = 64 as u32;

        let size = (width * height) as usize;
        let mut cells = FixedBitSet::with_capacity(size);

        for i in 0..size {
            cells.set(i, false);
        }

        // Glider
        // . 1 .
        // . . 1
        // 1 1 1
        let spaceship = [(1, 0), (2, 1), (0, 2), (1, 2), (2, 2)];

        for (row, col) in spaceship.iter().cloned() {
            let idx = row * width + col;
            cells.set(idx as usize, true);
        }

        Universe {
            width,
            height,
            cells,
            debug: false,
        }
    }

    pub fn new_random() -> Universe {
        let width = 64;
        let height = 64;

        let size = (width * height) as usize;
        let mut cells = FixedBitSet::with_capacity(size);

        for i in 0..size {
            cells.set(i, Math::random() < 0.5);
        }

        Universe {
            width,
            height,
            cells,
            debug: false,
        }
    }

    pub fn set_width(&mut self, width: u32) {
        self.width = width;
        let size = (self.width * self.height) as usize;
        let mut cells = FixedBitSet::with_capacity(size);

        for i in 0..size {
            cells.set(i, false)
        }

        self.cells = cells;
    }

    pub fn width(&self) -> u32 {
        return self.width;
    }

    pub fn set_height(&mut self, height: u32) {
        self.height = height;
        let size = (self.width * self.height) as usize;
        let mut cells = FixedBitSet::with_capacity(size);

        for i in 0..size {
            cells.set(i, false)
        }

        self.cells = cells;
    }

    pub fn height(&self) -> u32 {
        return self.height;
    }

    pub fn cells(&self) -> *const u32 {
        self.cells.as_slice().as_ptr()
    }

    pub fn render(&self) -> String {
        self.to_string()
    }

    pub fn tick(&mut self) {
        let _timer = Timer::new("Universe::tick");
        let mut next = {
            let _timer = Timer::new("allocate next cells");
            self.cells.clone()
        };

        {
            let _timer = Timer::new("new generation");
            for row in 0..self.height {
                for col in 0..self.width {
                    let idx = self.get_index(row, col);
                    let cell = self.cells[idx];
                    let live_neighbors = self.live_neighbor_count(row, col);

                    next.set(
                        idx,
                        match (cell, live_neighbors) {
                            // Any live cell with < 2 neighbors dies
                            (true, x) if x < 2 => {
                                if self.debug {
                                    log!("{:?} dies to loneliness", self.get_pos(idx));
                                }
                                false
                            }
                            // Any live cell with 2-3 neighbors survives
                            (true, 2) | (true, 3) => true,
                            // Any live cell with > 3 neighbors dies
                            (true, x) if x > 3 => {
                                if self.debug {
                                    log!("{:?} dies to overcrowding", self.get_pos(idx));
                                }
                                false
                            }
                            // Any dead cell with 3 neighbors becomes live
                            (false, 3) => {
                                if self.debug {
                                    log!("{:?} becomes live", self.get_pos(idx));
                                }
                                true
                            }
                            // Retain same state
                            (orig, _) => orig,
                        },
                    );
                }
            }
        }

        let _timer = Timer::new("free old cells");
        self.cells = next;
    }

    pub fn toggle_cell(&mut self, row: u32, column: u32) {
        let idx = self.get_index(row, column);
        self.cells.set(idx, !self.cells[idx]);
    }

    pub fn clear_cells(&mut self) {
        let size = (self.width * self.height) as usize;
        for i in 0..size {
            self.cells.set(i, false);
        }
    }

    pub fn add_glider_at_point(&mut self, row: u32, column: u32) {
        let glider = [(1, 0), (2, 1), (0, 2), (1, 2), (2, 2)];

        for (delta_row, delta_col) in glider.iter().cloned() {
            let target_row = (row + delta_row) % self.height;
            let target_col = (column + delta_col) % self.width;
            let idx = target_row * self.width + target_col;
            self.cells.set(idx as usize, true);
        }
    }

    pub fn add_pulsar_at_point(&mut self, row: u32, column: u32) {
        // TODO: Would programatically creating the mirrored quadrants
        // actually be simpler than defining every point? Maybe
        let pulsar = [
            (0, 4),
            (0, 10),
            (1, 4),
            (1, 10),
            (2, 4),
            (2, 5),
            (2, 9),
            (2, 10),
            (4, 0),
            (4, 1),
            (4, 2),
            (4, 5),
            (4, 6),
            (4, 8),
            (4, 9),
            (4, 12),
            (4, 13),
            (4, 14),
            (5, 2),
            (5, 4),
            (5, 6),
            (5, 8),
            (5, 10),
            (5, 12),
            (6, 4),
            (6, 5),
            (6, 9),
            (6, 10),
            (8, 4),
            (8, 5),
            (8, 9),
            (8, 10),
            (9, 2),
            (9, 4),
            (9, 6),
            (9, 8),
            (9, 10),
            (9, 12),
            (10, 0),
            (10, 1),
            (10, 2),
            (10, 5),
            (10, 6),
            (10, 8),
            (10, 9),
            (10, 12),
            (10, 13),
            (10, 14),
            (12, 4),
            (12, 5),
            (12, 9),
            (12, 10),
            (13, 4),
            (13, 10),
            (14, 4),
            (14, 10),
        ];

        for (delta_row, delta_col) in pulsar.iter().cloned() {
            let target_row = (row + delta_row) % self.height;
            let target_col = (column + delta_col) % self.width;
            let idx = target_row * self.width + target_col;
            self.cells.set(idx as usize, true);
        }
    }

    pub fn set_debug(&mut self, debug: bool) {
        self.debug = debug;
    }
}

impl fmt::Display for Universe {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let cells = &self.cells;
        for row in 0..self.height {
            for col in 0..self.width {
                let i = self.get_index(row, col);
                let symbol = if cells[i] { '◻' } else { '◼' };
                write!(f, "{}", symbol)?;
            }
            write!(f, "\n")?;
        }
        Ok(())
    }
}
