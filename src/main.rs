use macroquad::prelude::*;

const COLS: usize = 10;
const ROWS: usize = 20;
const CELL: f32 = 32.0;
const BOARD_X: f32 = 200.0;
const BOARD_Y: f32 = 40.0;

// テトロミノの形状定義 (4x4グリッド)
const PIECES: [[[u8; 4]; 4]; 7] = [
    // I
    [
        [0, 0, 0, 0],
        [1, 1, 1, 1],
        [0, 0, 0, 0],
        [0, 0, 0, 0],
    ],
    // O
    [
        [0, 1, 1, 0],
        [0, 1, 1, 0],
        [0, 0, 0, 0],
        [0, 0, 0, 0],
    ],
    // T
    [
        [0, 1, 0, 0],
        [1, 1, 1, 0],
        [0, 0, 0, 0],
        [0, 0, 0, 0],
    ],
    // S
    [
        [0, 1, 1, 0],
        [1, 1, 0, 0],
        [0, 0, 0, 0],
        [0, 0, 0, 0],
    ],
    // Z
    [
        [1, 1, 0, 0],
        [0, 1, 1, 0],
        [0, 0, 0, 0],
        [0, 0, 0, 0],
    ],
    // J
    [
        [1, 0, 0, 0],
        [1, 1, 1, 0],
        [0, 0, 0, 0],
        [0, 0, 0, 0],
    ],
    // L
    [
        [0, 0, 1, 0],
        [1, 1, 1, 0],
        [0, 0, 0, 0],
        [0, 0, 0, 0],
    ],
];

const COLORS: [Color; 7] = [
    SKYBLUE, YELLOW, PURPLE, GREEN, RED, BLUE, ORANGE,
];

fn rotate(piece: &[[u8; 4]; 4]) -> [[u8; 4]; 4] {
    let mut rotated = [[0u8; 4]; 4];
    for r in 0..4 {
        for c in 0..4 {
            rotated[c][3 - r] = piece[r][c];
        }
    }
    rotated
}

struct Piece {
    shape: [[u8; 4]; 4],
    color: Color,
    x: i32,
    y: i32,
}

impl Piece {
    fn new(kind: usize) -> Self {
        Self {
            shape: PIECES[kind],
            color: COLORS[kind],
            x: (COLS as i32 / 2) - 2,
            y: 0,
        }
    }

    fn cells(&self) -> impl Iterator<Item = (i32, i32)> + '_ {
        (0..4).flat_map(move |r| {
            (0..4).filter_map(move |c| {
                if self.shape[r][c] != 0 {
                    Some((self.x + c as i32, self.y + r as i32))
                } else {
                    None
                }
            })
        })
    }
}

struct Board {
    cells: [[Option<Color>; COLS]; ROWS],
}

impl Board {
    fn new() -> Self {
        Self {
            cells: [[None; COLS]; ROWS],
        }
    }

    fn collides(&self, piece: &Piece) -> bool {
        for (cx, cy) in piece.cells() {
            if cx < 0 || cx >= COLS as i32 || cy >= ROWS as i32 {
                return true;
            }
            if cy >= 0 && self.cells[cy as usize][cx as usize].is_some() {
                return true;
            }
        }
        false
    }

    fn lock(&mut self, piece: &Piece) {
        for (cx, cy) in piece.cells() {
            if cy >= 0 {
                self.cells[cy as usize][cx as usize] = Some(piece.color);
            }
        }
    }

    fn clear_lines(&mut self) -> u32 {
        let mut cleared = 0u32;
        let mut new_cells = [[None; COLS]; ROWS];
        let mut write_row = ROWS - 1;

        for r in (0..ROWS).rev() {
            if self.cells[r].iter().all(|c| c.is_some()) {
                cleared += 1;
            } else {
                new_cells[write_row] = self.cells[r];
                if write_row > 0 {
                    write_row -= 1;
                }
            }
        }
        self.cells = new_cells;
        cleared
    }

    fn draw(&self) {
        draw_rectangle_lines(
            BOARD_X - 2.0,
            BOARD_Y - 2.0,
            COLS as f32 * CELL + 4.0,
            ROWS as f32 * CELL + 4.0,
            2.0,
            WHITE,
        );

        for r in 0..ROWS {
            for c in 0..COLS {
                let x = BOARD_X + c as f32 * CELL;
                let y = BOARD_Y + r as f32 * CELL;
                if let Some(color) = self.cells[r][c] {
                    draw_rectangle(x, y, CELL - 1.0, CELL - 1.0, color);
                } else {
                    draw_rectangle_lines(
                        x, y, CELL - 1.0, CELL - 1.0, 0.5,
                        Color::new(0.2, 0.2, 0.2, 1.0),
                    );
                }
            }
        }
    }
}

fn draw_piece(piece: &Piece) {
    for (cx, cy) in piece.cells() {
        if cy >= 0 {
            let x = BOARD_X + cx as f32 * CELL;
            let y = BOARD_Y + cy as f32 * CELL;
            draw_rectangle(x, y, CELL - 1.0, CELL - 1.0, piece.color);
        }
    }
}

fn ghost_y(board: &Board, piece: &Piece) -> i32 {
    let mut gy = piece.y;
    loop {
        let test = Piece {
            shape: piece.shape,
            color: piece.color,
            x: piece.x,
            y: gy + 1,
        };
        if board.collides(&test) {
            break;
        }
        gy += 1;
    }
    gy
}

fn draw_ghost(board: &Board, piece: &Piece) {
    let gy = ghost_y(board, piece);
    let ghost = Piece {
        shape: piece.shape,
        color: piece.color,
        x: piece.x,
        y: gy,
    };
    for (cx, cy) in ghost.cells() {
        if cy >= 0 {
            let x = BOARD_X + cx as f32 * CELL;
            let y = BOARD_Y + cy as f32 * CELL;
            draw_rectangle_lines(
                x, y, CELL - 1.0, CELL - 1.0, 1.5,
                Color::new(piece.color.r, piece.color.g, piece.color.b, 0.4),
            );
        }
    }
}

fn score_for(lines: u32) -> u32 {
    match lines {
        1 => 100,
        2 => 300,
        3 => 500,
        4 => 800,
        _ => 0,
    }
}

fn next_kind() -> usize {
    rand::gen_range(0, 7)
}

struct Game {
    board: Board,
    current: Piece,
    next_kind: usize,
    score: u32,
    lines: u32,
    level: u32,
    fall_timer: f32,
    lock_timer: f32,
    game_over: bool,
    das_timer: f32,
    das_active: bool,
    last_dir: i32,
}

impl Game {
    fn new() -> Self {
        let kind = next_kind();
        let nk = next_kind();
        Self {
            board: Board::new(),
            current: Piece::new(kind),
            next_kind: nk,
            score: 0,
            lines: 0,
            level: 1,
            fall_timer: 0.0,
            lock_timer: 0.0,
            game_over: false,
            das_timer: 0.0,
            das_active: false,
            last_dir: 0,
        }
    }

    fn fall_interval(&self) -> f32 {
        (0.8 - (self.level as f32 - 1.0) * 0.007).max(0.05)
    }

    fn try_move(&mut self, dx: i32, dy: i32) -> bool {
        self.current.x += dx;
        self.current.y += dy;
        if self.board.collides(&self.current) {
            self.current.x -= dx;
            self.current.y -= dy;
            false
        } else {
            true
        }
    }

    fn try_rotate(&mut self) {
        let rotated = rotate(&self.current.shape);
        let old_shape = self.current.shape;
        self.current.shape = rotated;
        // wall kick
        for kick in [0, -1, 1, -2, 2] {
            self.current.x += kick;
            if !self.board.collides(&self.current) {
                return;
            }
            self.current.x -= kick;
        }
        self.current.shape = old_shape;
    }

    fn hard_drop(&mut self) {
        let gy = ghost_y(&self.board, &self.current);
        let dropped = gy - self.current.y;
        self.current.y = gy;
        self.score += dropped as u32 * 2;
        self.lock_piece();
    }

    fn lock_piece(&mut self) {
        self.board.lock(&self.current);
        let cleared = self.board.clear_lines();
        self.lines += cleared;
        self.score += score_for(cleared) * self.level;
        self.level = self.lines / 10 + 1;

        let kind = self.next_kind;
        self.next_kind = next_kind();
        self.current = Piece::new(kind);
        self.fall_timer = 0.0;
        self.lock_timer = 0.0;

        if self.board.collides(&self.current) {
            self.game_over = true;
        }
    }

    fn update(&mut self, dt: f32) {
        if self.game_over {
            return;
        }

        // 横移動 (DAS: Delayed Auto Shift)
        let dir = if is_key_down(KeyCode::Left) {
            -1
        } else if is_key_down(KeyCode::Right) {
            1
        } else {
            0
        };

        if dir != 0 {
            if dir != self.last_dir {
                self.try_move(dir, 0);
                self.das_timer = 0.0;
                self.das_active = false;
                self.last_dir = dir;
            } else {
                self.das_timer += dt;
                let threshold = if self.das_active { 0.05 } else { 0.15 };
                if self.das_timer >= threshold {
                    self.try_move(dir, 0);
                    self.das_timer = 0.0;
                    self.das_active = true;
                }
            }
        } else {
            self.last_dir = 0;
            self.das_active = false;
            self.das_timer = 0.0;
        }

        // 落下間隔 (ソフトドロップ中は加速)
        let interval = if is_key_down(KeyCode::Down) {
            0.05f32
        } else {
            self.fall_interval()
        };

        self.fall_timer += dt;
        if self.fall_timer >= interval {
            self.fall_timer = 0.0;
            if !self.try_move(0, 1) {
                self.lock_timer += interval;
                if self.lock_timer >= 0.5 {
                    self.lock_piece();
                }
            } else {
                self.lock_timer = 0.0;
            }
        }
    }

    fn handle_keys(&mut self) {
        if self.game_over {
            if is_key_pressed(KeyCode::R) {
                *self = Game::new();
            }
            return;
        }

        if is_key_pressed(KeyCode::Up) || is_key_pressed(KeyCode::X) {
            self.try_rotate();
        }
        if is_key_pressed(KeyCode::Space) {
            self.hard_drop();
        }
    }

    fn draw(&self) {
        self.board.draw();
        draw_ghost(&self.board, &self.current);
        draw_piece(&self.current);

        // NEXTピース
        let nx = BOARD_X + COLS as f32 * CELL + 30.0;
        draw_text("NEXT", nx, BOARD_Y + 20.0, 22.0, WHITE);
        let next_piece = Piece::new(self.next_kind);
        for r in 0..4 {
            for c in 0..4 {
                if next_piece.shape[r][c] != 0 {
                    draw_rectangle(
                        nx + c as f32 * 28.0,
                        BOARD_Y + 40.0 + r as f32 * 28.0,
                        27.0,
                        27.0,
                        next_piece.color,
                    );
                }
            }
        }

        // スコア・ライン・レベル
        draw_text("SCORE", nx, BOARD_Y + 180.0, 22.0, WHITE);
        draw_text(&format!("{}", self.score), nx, BOARD_Y + 205.0, 22.0, YELLOW);
        draw_text("LINES", nx, BOARD_Y + 240.0, 22.0, WHITE);
        draw_text(&format!("{}", self.lines), nx, BOARD_Y + 265.0, 22.0, YELLOW);
        draw_text("LEVEL", nx, BOARD_Y + 300.0, 22.0, WHITE);
        draw_text(&format!("{}", self.level), nx, BOARD_Y + 325.0, 22.0, YELLOW);

        // 操作説明
        let lx = 10.0;
        draw_text("← → : 移動", lx, BOARD_Y + 60.0, 18.0, GRAY);
        draw_text("↑ / X: 回転", lx, BOARD_Y + 85.0, 18.0, GRAY);
        draw_text("↓ : 落下", lx, BOARD_Y + 110.0, 18.0, GRAY);
        draw_text("SPC: ハードドロップ", lx, BOARD_Y + 135.0, 18.0, GRAY);

        if self.game_over {
            let gw = screen_width();
            let gh = screen_height();
            draw_rectangle(
                gw * 0.2, gh * 0.35, gw * 0.6, gh * 0.3,
                Color::new(0.0, 0.0, 0.0, 0.85),
            );
            draw_text("GAME OVER", gw * 0.28, gh * 0.5, 48.0, RED);
            draw_text("R: リスタート", gw * 0.33, gh * 0.58, 28.0, WHITE);
        }
    }
}

fn window_conf() -> Conf {
    Conf {
        window_title: "Tetris".to_string(),
        window_width: 620,
        window_height: 720,
        window_resizable: false,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut game = Game::new();

    loop {
        let dt = get_frame_time();
        clear_background(Color::new(0.05, 0.05, 0.1, 1.0));

        game.handle_keys();
        game.update(dt);
        game.draw();

        next_frame().await;
    }
}
