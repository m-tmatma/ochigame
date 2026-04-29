#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use macroquad::prelude::*;

// ボードのサイズ (標準テトリス: 横10, 縦20)
const COLS: usize = 10;
const ROWS: usize = 20;
// 1セルのピクセルサイズ
const CELL: f32 = 32.0;
// ボード左上の描画位置
const BOARD_X: f32 = 200.0;
const BOARD_Y: f32 = 40.0;
// ライン消去爆発アニメーションの時間 (秒)
const CLEAR_ANIM_DURATION: f32 = 0.45;
// 重力アニメーションの落下速度 (行/秒) — 小さいほどゆっくり
const FALL_SPEED: f32 = 5.0;

// テトロミノ7種の形状。各ピースは4x4のビットマップで表現し、
// 1 がブロックのあるマス、0 が空マス。
// 初期向きは SRS (Super Rotation System) の spawn 状態に準拠。
const PIECES: [[[u8; 4]; 4]; 7] = [
    // I — 横一列4マス
    [
        [0, 0, 0, 0],
        [1, 1, 1, 1],
        [0, 0, 0, 0],
        [0, 0, 0, 0],
    ],
    // O — 2x2正方形
    [
        [0, 1, 1, 0],
        [0, 1, 1, 0],
        [0, 0, 0, 0],
        [0, 0, 0, 0],
    ],
    // T — T字型
    [
        [0, 1, 0, 0],
        [1, 1, 1, 0],
        [0, 0, 0, 0],
        [0, 0, 0, 0],
    ],
    // S — 右上がりZ字
    [
        [0, 1, 1, 0],
        [1, 1, 0, 0],
        [0, 0, 0, 0],
        [0, 0, 0, 0],
    ],
    // Z — 左上がりZ字
    [
        [1, 1, 0, 0],
        [0, 1, 1, 0],
        [0, 0, 0, 0],
        [0, 0, 0, 0],
    ],
    // J — 左L字
    [
        [1, 0, 0, 0],
        [1, 1, 1, 0],
        [0, 0, 0, 0],
        [0, 0, 0, 0],
    ],
    // L — 右L字
    [
        [0, 0, 1, 0],
        [1, 1, 1, 0],
        [0, 0, 0, 0],
        [0, 0, 0, 0],
    ],
];

// 各テトロミノに対応する色 (PIECES と同じインデックス順)
const COLORS: [Color; 7] = [
    SKYBLUE, // I
    YELLOW,  // O
    PURPLE,  // T
    GREEN,   // S
    RED,     // Z
    BLUE,    // J
    ORANGE,  // L
];

// 4x4形状を時計回りに90度回転させる。
// 変換式: rotated[c][3-r] = original[r][c]
fn rotate(piece: &[[u8; 4]; 4]) -> [[u8; 4]; 4] {
    let mut rotated = [[0u8; 4]; 4];
    for r in 0..4 {
        for c in 0..4 {
            rotated[c][3 - r] = piece[r][c];
        }
    }
    rotated
}

// 爆発パーティクル1個。セルの色を持ち、重力・空気抵抗で飛散する。
struct Particle {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    color: Color,
    size: f32,
    age: f32,
    lifetime: f32,
}

impl Particle {
    fn update(&mut self, dt: f32) {
        self.x += self.vx * dt;
        self.y += self.vy * dt;
        self.vy += 600.0 * dt; // 重力
        self.vx *= 1.0 - dt * 2.5; // 横方向の空気抵抗
        self.age += dt;
    }

    fn is_dead(&self) -> bool {
        self.age >= self.lifetime
    }

    fn draw(&self) {
        let t = (self.age / self.lifetime).min(1.0);
        let alpha = 1.0 - t;
        let size = self.size * (1.0 - t * 0.6); // 飛ぶにつれて小さくなる
        let color = Color::new(self.color.r, self.color.g, self.color.b, alpha);
        draw_rectangle(self.x - size * 0.5, self.y - size * 0.5, size, size, color);
    }
}

// ライン消去後にゆっくり落下するブロック1個。
// board に既に最終位置が書き込まれているが、visual_y がそこに到達するまで
// 目的地セルを隠してこちらを描画することで落下アニメーションを表現する。
struct FallingBlock {
    col: usize,
    color: Color,
    visual_y: f32,   // 現在の描画行 (小数、落下中に増加)
    target_row: usize, // 到達先の行
}

impl FallingBlock {
    fn is_done(&self) -> bool {
        self.visual_y >= self.target_row as f32
    }

    fn draw(&self) {
        let x = BOARD_X + self.col as f32 * CELL;
        let y = BOARD_Y + self.visual_y * CELL;
        draw_rectangle(x, y, CELL - 1.0, CELL - 1.0, self.color);
    }
}

// 操作中のテトロミノ1個を表す。
// x, y はボード座標系 (左上が原点、下がY正方向)。
struct Piece {
    shape: [[u8; 4]; 4],
    color: Color,
    x: i32, // ボード上の列位置 (4x4グリッドの左端)
    y: i32, // ボード上の行位置 (4x4グリッドの上端)
}

impl Piece {
    // 種類番号 (0–6) からピースを生成し、ボード上部中央に配置する。
    fn new(kind: usize) -> Self {
        Self {
            shape: PIECES[kind],
            color: COLORS[kind],
            x: (COLS as i32 / 2) - 2, // 4x4グリッドの中心をボード中央に合わせる
            y: 0,
        }
    }

    // 現在の形状・位置からブロックのあるセルのボード座標を列挙する。
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

// 積み上がったブロックを管理するボード。
// None = 空、Some(Color) = 固定済みブロック。
struct Board {
    cells: [[Option<Color>; COLS]; ROWS],
}

impl Board {
    fn new() -> Self {
        Self {
            cells: [[None; COLS]; ROWS],
        }
    }

    // ピースが壁・床・固定ブロックと重なっているか判定する。
    // y < 0 (ボード上端より上) は壁判定しない — スポーン直後に半分隠れるため。
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

    // ピースをボードに固定する。ボード上端より上のブロックは無視する。
    fn lock(&mut self, piece: &Piece) {
        for (cx, cy) in piece.cells() {
            if cy >= 0 {
                self.cells[cy as usize][cx as usize] = Some(piece.color);
            }
        }
    }

    // 揃っている行のインデックスを返す (消去はしない)。
    fn full_rows(&self) -> Vec<usize> {
        (0..ROWS)
            .filter(|&r| self.cells[r].iter().all(|c| c.is_some()))
            .collect()
    }

    // 指定した行のセルをすべて None にする。
    fn clear_rows(&mut self, rows: &[usize]) {
        for &r in rows {
            self.cells[r] = [None; COLS];
        }
    }

    // 各列ごとにブロックを独立して落下させる。
    // 消去によって生じた空白ギャップを埋め、浮いたブロックを着地させる。
    fn apply_gravity(&mut self) {
        for c in 0..COLS {
            let filled: Vec<Color> = (0..ROWS)
                .filter_map(|r| self.cells[r][c])
                .collect();
            let empty_rows = ROWS - filled.len();
            for r in 0..ROWS {
                self.cells[r][c] = if r < empty_rows {
                    None
                } else {
                    Some(filled[r - empty_rows])
                };
            }
        }
    }

    // ボードを描画する。
    // clearing_rows の行は白フラッシュ、hidden_cells のセルは FallingBlock が代わりに描画するため省略する。
    fn draw(&self, clearing_rows: &[usize], clear_timer: f32, hidden_cells: &[(usize, usize)]) {
        // ボードの外枠
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

                if clearing_rows.contains(&r) {
                    // 爆発直後の白フラッシュ: t=0.3 付近で完全透明になる
                    let t = clear_timer / CLEAR_ANIM_DURATION;
                    let alpha = (1.0 - t * 3.5).max(0.0);
                    if alpha > 0.0 {
                        draw_rectangle(x, y, CELL - 1.0, CELL - 1.0,
                            Color::new(1.0, 1.0, 1.0, alpha));
                    }
                } else if hidden_cells.contains(&(c, r)) {
                    // FallingBlock がアニメーション中のセルは描画しない
                    draw_rectangle_lines(x, y, CELL - 1.0, CELL - 1.0, 0.5,
                        Color::new(0.2, 0.2, 0.2, 1.0));
                } else if let Some(color) = self.cells[r][c] {
                    draw_rectangle(x, y, CELL - 1.0, CELL - 1.0, color);
                } else {
                    draw_rectangle_lines(x, y, CELL - 1.0, CELL - 1.0, 0.5,
                        Color::new(0.2, 0.2, 0.2, 1.0));
                }
            }
        }
    }
}

// 操作中のピースをボード上に描画する。
// ボード上端より上にあるセル (cy < 0) は描画しない。
fn draw_piece(piece: &Piece) {
    for (cx, cy) in piece.cells() {
        if cy >= 0 {
            let x = BOARD_X + cx as f32 * CELL;
            let y = BOARD_Y + cy as f32 * CELL;
            draw_rectangle(x, y, CELL - 1.0, CELL - 1.0, piece.color);
        }
    }
}

// ピースを1マスずつ下にずらしながら衝突判定し、着地Y座標を求める。
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

// ゴーストピース (落下先の輪郭) を半透明の枠線で描画する。
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

// 一度に消したライン数に応じたベーススコアを返す。
// テトリス (4ライン) は単純な4倍より大きくボーナスを付ける。
fn score_for(lines: u32) -> u32 {
    match lines {
        1 => 100,
        2 => 300,
        3 => 500,
        4 => 800, // テトリス: 4倍以上のボーナス
        _ => 0,
    }
}

// 0–6 のランダムなピース種別を返す。
fn next_kind() -> usize {
    rand::gen_range(0, 7)
}

// ゲーム全体の状態を保持する。
struct Game {
    board: Board,
    current: Piece,   // 現在操作中のピース
    next_kind: usize, // 次に出るピースの種類
    score: u32,
    lines: u32,
    level: u32,
    fall_timer: f32, // 次の自動落下までの経過時間 (秒)
    lock_timer: f32, // 着地後の固定までの待機時間 (秒)
    game_over: bool,
    // DAS (Delayed Auto Shift): キー長押しによる連続横移動の制御
    das_timer: f32,
    das_active: bool,
    last_dir: i32,
    // ライン消去爆発アニメーション
    clearing_rows: Vec<usize>,
    clear_anim_timer: f32,
    // 爆発パーティクル
    particles: Vec<Particle>,
    // 重力落下アニメーション
    falling_blocks: Vec<FallingBlock>,
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
            clearing_rows: Vec::new(),
            clear_anim_timer: 0.0,
            particles: Vec::new(),
            falling_blocks: Vec::new(),
        }
    }

    // レベルに応じた自動落下間隔 (秒) を返す。
    // レベル1で0.8秒、上がるごとに短くなり最速0.05秒で下限。
    fn fall_interval(&self) -> f32 {
        (0.8 - (self.level as f32 - 1.0) * 0.007).max(0.05)
    }

    // ピースを (dx, dy) だけ移動し、衝突するなら元に戻して false を返す。
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

    // 時計回りに回転を試み、壁キックで ±1, ±2 列もトライする。
    // すべて失敗したら回転をキャンセルする。
    fn try_rotate(&mut self) {
        let rotated = rotate(&self.current.shape);
        let old_shape = self.current.shape;
        self.current.shape = rotated;
        for kick in [0, -1, 1, -2, 2] {
            self.current.x += kick;
            if !self.board.collides(&self.current) {
                return;
            }
            self.current.x -= kick;
        }
        self.current.shape = old_shape;
    }

    // スペースキーによるハードドロップ。ゴースト位置まで瞬時に落とし固定する。
    fn hard_drop(&mut self) {
        let gy = ghost_y(&self.board, &self.current);
        let dropped = gy - self.current.y;
        self.current.y = gy;
        self.score += dropped as u32 * 2;
        self.lock_piece();
    }

    // ピースをボードに固定し、爆発アニメーションを開始する。
    fn lock_piece(&mut self) {
        self.board.lock(&self.current);
        self.begin_clear();
    }

    // 揃っている行を探し、あれば爆発パーティクルを生成してアニメーション開始。
    fn begin_clear(&mut self) {
        let rows = self.board.full_rows();
        if rows.is_empty() {
            self.spawn_next();
        } else {
            self.spawn_explosion_particles(&rows);
            self.clearing_rows = rows;
            self.clear_anim_timer = 0.0;
        }
    }

    // clearing_rows の各セルから爆発パーティクルを生成する。
    fn spawn_explosion_particles(&mut self, rows: &[usize]) {
        for &r in rows {
            for c in 0..COLS {
                if let Some(cell_color) = self.board.cells[r][c] {
                    let cx = BOARD_X + c as f32 * CELL + CELL * 0.5;
                    let cy = BOARD_Y + r as f32 * CELL + CELL * 0.5;
                    for i in 0..8 {
                        let base_angle = (i as f32 / 8.0) * std::f32::consts::PI * 2.0;
                        let angle = base_angle + rand::gen_range(-0.4f32, 0.4);
                        let speed = rand::gen_range(120.0f32, 480.0);
                        let vx = angle.cos() * speed;
                        let vy = angle.sin() * speed - rand::gen_range(0.0f32, 80.0);
                        let color = if rand::gen_range(0u32, 4) == 0 {
                            Color::new(1.0, 0.95, 0.7, 1.0)
                        } else {
                            cell_color
                        };
                        self.particles.push(Particle {
                            x: cx + rand::gen_range(-6.0f32, 6.0),
                            y: cy + rand::gen_range(-6.0f32, 6.0),
                            vx,
                            vy,
                            color,
                            size: rand::gen_range(4.0f32, 11.0),
                            age: 0.0,
                            lifetime: rand::gen_range(0.4f32, 0.85),
                        });
                    }
                }
            }
        }
    }

    // 爆発アニメーション終了: 行を消去し、落下アニメーションを開始する。
    fn finish_clear(&mut self) {
        let count = self.clearing_rows.len() as u32;
        self.board.clear_rows(&self.clearing_rows);
        self.clearing_rows.clear();

        // 重力適用前後の差分から FallingBlock を生成する
        let before = self.board.cells; // コピー
        self.board.apply_gravity();

        self.falling_blocks.clear();
        for c in 0..COLS {
            // 消去前のこの列のブロック (上から順)
            let cells_before: Vec<(usize, Color)> = (0..ROWS)
                .filter_map(|r| before[r][c].map(|color| (r, color)))
                .collect();
            let n = cells_before.len();
            let target_start = ROWS - n; // 重力後の先頭行
            for (i, (from_row, color)) in cells_before.iter().enumerate() {
                let to_row = target_start + i;
                if *from_row != to_row {
                    self.falling_blocks.push(FallingBlock {
                        col: c,
                        color: *color,
                        visual_y: *from_row as f32,
                        target_row: to_row,
                    });
                }
            }
        }

        self.lines += count;
        self.score += score_for(count) * self.level;
        self.level = self.lines / 10 + 1;

        // 落下するブロックがなければ即座に連鎖チェックへ
        if self.falling_blocks.is_empty() {
            self.begin_clear();
        }
    }

    // 落下アニメーション終了: 連鎖チェックまたは次ピースへ。
    fn finish_fall(&mut self) {
        self.falling_blocks.clear();
        self.begin_clear();
    }

    // 次のピースをスポーンする。衝突していればゲームオーバー。
    fn spawn_next(&mut self) {
        let kind = self.next_kind;
        self.next_kind = next_kind();
        self.current = Piece::new(kind);
        self.fall_timer = 0.0;
        self.lock_timer = 0.0;
        if self.board.collides(&self.current) {
            self.game_over = true;
        }
    }

    // 毎フレーム呼ばれる更新処理。dt は前フレームからの経過時間 (秒)。
    fn update(&mut self, dt: f32) {
        if self.game_over {
            return;
        }

        // パーティクルはゲーム状態に関係なく常に更新する
        for p in &mut self.particles {
            p.update(dt);
        }
        self.particles.retain(|p| !p.is_dead());

        // ── 状態: 爆発アニメーション ─────────────────────────────────
        if !self.clearing_rows.is_empty() {
            self.clear_anim_timer += dt;
            if self.clear_anim_timer >= CLEAR_ANIM_DURATION {
                self.finish_clear();
            }
            return;
        }

        // ── 状態: 落下アニメーション ─────────────────────────────────
        if !self.falling_blocks.is_empty() {
            for fb in &mut self.falling_blocks {
                fb.visual_y = (fb.visual_y + FALL_SPEED * dt)
                    .min(fb.target_row as f32);
            }
            if self.falling_blocks.iter().all(|fb| fb.is_done()) {
                self.finish_fall();
            }
            return;
        }

        // ── 横移動 (DAS: Delayed Auto Shift) ──────────────────────────
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

        // ── 自動落下 ──────────────────────────────────────────────────
        let interval = if is_key_down(KeyCode::Down) { 0.05f32 } else { self.fall_interval() };

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

        // アニメーション中は操作を受け付けない
        if !self.clearing_rows.is_empty() || !self.falling_blocks.is_empty() {
            return;
        }

        if is_key_pressed(KeyCode::Up) || is_key_pressed(KeyCode::X) {
            self.try_rotate();
        }
        if is_key_pressed(KeyCode::Space) {
            self.hard_drop();
        }
    }

    fn draw(&self, font: &Font) {
        // 落下アニメーション中は目的地セルを隠す (FallingBlock が代わりに描画)
        let hidden: Vec<(usize, usize)> = self.falling_blocks
            .iter()
            .map(|fb| (fb.col, fb.target_row))
            .collect();
        self.board.draw(&self.clearing_rows, self.clear_anim_timer, &hidden);

        // アニメーション中はピース・ゴーストを描画しない
        if self.clearing_rows.is_empty() && self.falling_blocks.is_empty() {
            draw_ghost(&self.board, &self.current);
            draw_piece(&self.current);
        }

        // 落下中のブロックを描画
        for fb in &self.falling_blocks {
            fb.draw();
        }

        // パーティクルを描画
        for p in &self.particles {
            p.draw();
        }

        // ── NEXTピースのプレビュー ────────────────────────────────────
        let nx = BOARD_X + COLS as f32 * CELL + 30.0;
        draw_text("NEXT", nx, BOARD_Y + 20.0, 22.0, WHITE);
        let next_piece = Piece::new(self.next_kind);
        for r in 0..4 {
            for c in 0..4 {
                if next_piece.shape[r][c] != 0 {
                    draw_rectangle(
                        nx + c as f32 * 28.0,
                        BOARD_Y + 40.0 + r as f32 * 28.0,
                        27.0, 27.0,
                        next_piece.color,
                    );
                }
            }
        }

        // ── スコア・ライン・レベル ────────────────────────────────────
        draw_text("SCORE", nx, BOARD_Y + 180.0, 22.0, WHITE);
        draw_text(&format!("{}", self.score), nx, BOARD_Y + 205.0, 22.0, YELLOW);
        draw_text("LINES", nx, BOARD_Y + 240.0, 22.0, WHITE);
        draw_text(&format!("{}", self.lines), nx, BOARD_Y + 265.0, 22.0, YELLOW);
        draw_text("LEVEL", nx, BOARD_Y + 300.0, 22.0, WHITE);
        draw_text(&format!("{}", self.level), nx, BOARD_Y + 325.0, 22.0, YELLOW);

        // ── 操作説明 ──────────────────────────────────────────────────
        let lx = 10.0;
        let jp = |text, x, y, size, color| {
            draw_text_ex(text, x, y, TextParams { font: Some(font), font_size: size, color, ..Default::default() });
        };
        jp("← → : 移動", lx, BOARD_Y + 60.0, 18, GRAY);
        jp("↑ / X: 回転", lx, BOARD_Y + 85.0, 18, GRAY);
        jp("↓ : 落下", lx, BOARD_Y + 110.0, 18, GRAY);
        jp("SPC: ハードドロップ", lx, BOARD_Y + 135.0, 18, GRAY);

        // ── ゲームオーバー画面 ────────────────────────────────────────
        if self.game_over {
            let gw = screen_width();
            let gh = screen_height();
            draw_rectangle(
                gw * 0.2, gh * 0.35, gw * 0.6, gh * 0.3,
                Color::new(0.0, 0.0, 0.0, 0.85),
            );
            draw_text("GAME OVER", gw * 0.28, gh * 0.5, 48.0, RED);
            draw_text_ex("R: リスタート", gw * 0.33, gh * 0.58, TextParams { font: Some(font), font_size: 28, color: WHITE, ..Default::default() });
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
    let font = load_ttf_font_from_bytes(include_bytes!("../assets/NotoSansJP.ttf")).unwrap();
    let mut game = Game::new();

    loop {
        let dt = get_frame_time();
        clear_background(Color::new(0.05, 0.05, 0.1, 1.0));

        game.handle_keys();
        game.update(dt);
        game.draw(&font);

        next_frame().await;
    }
}
