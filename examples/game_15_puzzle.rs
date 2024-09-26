use rkit::app::{window_size, WindowConfig};
use rkit::draw::{create_draw_2d, Draw2D};
use rkit::gfx::{self, Color};
use rkit::input::{is_mouse_btn_pressed, mouse_position, MouseButton};
use rkit::math::{ivec2, uvec2, vec2, IVec2, UVec2, Vec2};
use rkit::random;

const COLS: usize = 4;
const NUMBERS: usize = COLS * COLS;
const TILE_SIZE: f32 = 100.0;
const BOARD_SIZE: f32 = COLS as f32 * TILE_SIZE;

// TODO multisampling

const FILL_COLOR: Color = Color::rgb(0.9, 0.9, 0.9);
const OUTLINE_COLOR: Color = Color::rgb(0.0, 0.8, 0.7);

fn main() -> Result<(), String> {
    let win = WindowConfig {
        size: uvec2(BOARD_SIZE as _, BOARD_SIZE as _),
        ..Default::default()
    };

    rkit::init_with(State::new)
        .with_window(win)
        .on_update(update)
        .run()
}

fn update(state: &mut State) {
    if is_mouse_btn_pressed(MouseButton::Left) {
        if let Some(pos) = mouse_pos_to_point() {
            if let Some(dir) = state.board.can_move_to(pos) {
                let t_pos = match dir {
                    Direction::Left => uvec2(pos.x - 1, pos.y),
                    Direction::Right => uvec2(pos.x + 1, pos.y),
                    Direction::Up => uvec2(pos.x, pos.y - 1),
                    Direction::Down => uvec2(pos.x, pos.y + 1),
                };

                state.board.move_tile(pos, t_pos);
                return;
            }
        }
    }

    if state.board.is_solved() && is_mouse_btn_pressed(MouseButton::Left) {
        state.reset();
    }

    draw(state);
}

fn mouse_pos_to_point() -> Option<UVec2> {
    let pos = mouse_position();
    let in_x_bounds = pos.x >= 0.0 && pos.x < TILE_SIZE * COLS as f32;
    let in_y_bounds = pos.y >= 0.0 && pos.y < TILE_SIZE * COLS as f32;
    let in_bounds = in_x_bounds && in_y_bounds;
    if !in_bounds {
        return None;
    }

    let point = (pos / TILE_SIZE).floor();
    Some(point.as_uvec2())
}

fn draw(state: &mut State) {
    let mut draw = create_draw_2d();
    draw.clear(Color::BLACK);

    for y in 0..COLS {
        for x in 0..COLS {
            let value = state.board.value(uvec2(x as _, y as _));
            draw_tile(&mut draw, x, y, value);
        }
    }

    if state.board.is_solved() {
        let size = window_size();

        draw.rect(Vec2::splat(0.0), size)
            .color(Color::BLACK)
            .alpha(0.7);

        draw.text("Done!")
            .color(Color::ORANGE)
            .size(74.0)
            .translate(size * 0.5)
            .anchor(vec2(0.5, 1.0));

        draw.text("Tap to reset")
            .color(Color::GRAY)
            .size(54.0)
            .anchor(vec2(0.5, 0.0))
            .translate(size * 0.5 + Vec2::Y * 20.0);
    }

    gfx::render_to_frame(&draw).unwrap();
}

fn draw_tile(draw: &mut Draw2D, x: usize, y: usize, value: u8) {
    if value == 0 {
        return;
    }

    let xx = x as f32 * TILE_SIZE;
    let yy = y as f32 * TILE_SIZE;
    draw.rect(vec2(xx, yy), vec2(TILE_SIZE, TILE_SIZE))
        .corner_radius(10.0)
        .fill_color(FILL_COLOR)
        .fill()
        .stroke_color(OUTLINE_COLOR)
        .stroke(5.0);

    draw.text(&format!("{value}"))
        .color(Color::BLACK)
        .size(34.0)
        .translate(vec2(xx + TILE_SIZE * 0.5, yy + TILE_SIZE * 0.5))
        .anchor(Vec2::splat(0.5));
}

struct State {
    board: Board,
}

impl State {
    fn new() -> Self {
        let board = Board::new();
        Self { board }
    }

    fn reset(&mut self) {
        self.board = Board::new();
    }
}

#[derive(Debug, Copy, Clone)]
enum Direction {
    Left,
    Right,
    Up,
    Down,
}

struct Board {
    grid: [u8; NUMBERS],
}

impl Board {
    /// Create a new board using random numbers
    fn new() -> Self {
        // generate an initial board, and move randomly from it.

        let mut grid = [0; NUMBERS];
        grid.iter_mut().enumerate().for_each(|(i, n)| {
            if i == NUMBERS - 1 {
                return;
            }

            *n = i as u8 + 1;
        });

        // move blank cell randomly.

        let mut pos = ivec2(COLS as _, COLS as _) - 1;
        for _ in 0..1000 {
            const DIRS: [IVec2; 4] = [ivec2(0, -1), ivec2(-1, 0), ivec2(1, 0), ivec2(0, 1)];
            let dir = random::pick(DIRS).unwrap();
            let nxt = pos + dir;

            const RANGE: std::ops::Range<i32> = 0..COLS as i32;
            if !RANGE.contains(&nxt.x) || !RANGE.contains(&nxt.y) {
                continue;
            }

            let index = index_from_point(pos.as_uvec2());
            let index_nxt = index_from_point(nxt.as_uvec2());
            grid.swap(index, index_nxt);

            pos = nxt;
        }

        Self { grid }
    }

    fn is_solved(&self) -> bool {
        for i in 0..NUMBERS - 1 {
            if self.grid[i] != i as u8 + 1 {
                return false;
            }
        }
        true
    }

    fn value(&self, pos: UVec2) -> u8 {
        let index = index_from_point(pos);
        self.grid[index]
    }

    fn is_empty(&self, pos: UVec2) -> bool {
        self.value(pos) == 0
    }

    fn move_tile(&mut self, pos1: UVec2, pos2: UVec2) {
        let index1 = index_from_point(pos1);
        let index2 = index_from_point(pos2);
        self.grid.swap(index1, index2);
    }

    fn can_move_to(&self, pos: UVec2) -> Option<Direction> {
        let UVec2 { x, y } = pos;
        let (x, y) = (x as usize, y as usize);

        if x >= 1 && self.is_empty(pos - UVec2::X) {
            return Some(Direction::Left);
        }

        if x <= COLS - 2 && self.is_empty(pos + UVec2::X) {
            return Some(Direction::Right);
        }

        if y >= 1 && self.is_empty(pos - UVec2::Y) {
            return Some(Direction::Up);
        }

        if y <= COLS - 2 && self.is_empty(pos + UVec2::Y) {
            return Some(Direction::Down);
        }

        None
    }
}

#[inline]
fn index_from_point(pos: UVec2) -> usize {
    let (x, y) = (pos.x as usize, pos.y as usize);
    debug_assert!(x < COLS || y < COLS, "Point index out of bounds.");
    y * COLS + x
}
