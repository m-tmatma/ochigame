use macroquad::prelude::*;

// ボードのサイズ (標準テトリス: 横10, 縦20)
const COLS: usize = 10;
const ROWS: usize = 20;
// 1セルのピクセルサイズ
const CELL: f32 = 32.0;
// ボード左上の描画位置
const BOARD_X: f32 = 200.0;
const BOARD_Y: f32 = 40.0;

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

    // 揃ったラインのセルをすべて None にして消去数を返す。
    // ブロックの落下は apply_gravity が担う。
    fn clear_lines(&mut self) -> u32 {
        let mut cleared = 0u32;
        for r in 0..ROWS {
            if self.cells[r].iter().all(|c| c.is_some()) {
                self.cells[r] = [None; COLS];
                cleared += 1;
            }
        }
        cleared
    }

    // ライン消去後、各列ごとにブロックを独立して落下させる。
    // 消去によって生じた空白ギャップを埋め、浮いたブロックを着地させる。
    fn apply_gravity(&mut self) {
        for c in 0..COLS {
            // この列の上から順に、Someのセルだけ集める (相対順序を保持)
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

    fn draw(&self) {
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
                if let Some(color) = self.cells[r][c] {
                    // 固定済みブロック
                    draw_rectangle(x, y, CELL - 1.0, CELL - 1.0, color);
                } else {
                    // 空セル: 薄いグリッド線だけ描画
                    draw_rectangle_lines(
                        x, y, CELL - 1.0, CELL - 1.0, 0.5,
                        Color::new(0.2, 0.2, 0.2, 1.0),
                    );
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
    current: Piece,  // 現在操作中のピース
    next_kind: usize, // 次に出るピースの種類
    score: u32,
    lines: u32,
    level: u32,
    fall_timer: f32,  // 次の自動落下までの経過時間 (秒)
    lock_timer: f32,  // 着地後の固定までの待機時間 (秒)
    game_over: bool,
    // DAS (Delayed Auto Shift): キー長押しによる連続横移動の制御
    das_timer: f32,  // 現在の方向キー押下継続時間
    das_active: bool, // 初回ディレイを超えてARR段階に入っているか
    last_dir: i32,   // 前フレームの方向 (-1/0/1)
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
        // SRS 簡易版: X軸方向に 0, -1, +1, -2, +2 のオフセットを試みる
        for kick in [0, -1, 1, -2, 2] {
            self.current.x += kick;
            if !self.board.collides(&self.current) {
                return; // この位置で回転成功
            }
            self.current.x -= kick;
        }
        // すべてのキックが失敗 → 回転前の形状に戻す
        self.current.shape = old_shape;
    }

    // スペースキーによるハードドロップ。ゴースト位置まで瞬時に落とし固定する。
    // 落下マス数 × 2 点をスコアに加算する。
    fn hard_drop(&mut self) {
        let gy = ghost_y(&self.board, &self.current);
        let dropped = gy - self.current.y;
        self.current.y = gy;
        self.score += dropped as u32 * 2;
        self.lock_piece();
    }

    // ピースをボードに固定し、ライン消去・スコア加算・次ピース生成を行う。
    // 新ピースがスポーン直後から衝突していればゲームオーバーとする。
    fn lock_piece(&mut self) {
        self.board.lock(&self.current);
        let mut total_cleared = 0u32;
        loop {
            let cleared = self.board.clear_lines();
            if cleared == 0 {
                break;
            }
            total_cleared += cleared;
            self.board.apply_gravity();
        }
        self.lines += total_cleared;
        let cleared = total_cleared;
        self.score += score_for(cleared) * self.level; // レベル倍率を掛ける
        self.level = self.lines / 10 + 1; // 10ライン毎にレベルアップ

        // 次のピースを現在に昇格し、新たに next を決める
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

        // ── 横移動 (DAS: Delayed Auto Shift) ──────────────────────────
        // 最初の押下で即時移動し、0.15秒後から 0.05秒間隔で連続移動する。
        let dir = if is_key_down(KeyCode::Left) {
            -1
        } else if is_key_down(KeyCode::Right) {
            1
        } else {
            0
        };

        if dir != 0 {
            if dir != self.last_dir {
                // 新たな方向キーが押された → 即時1回移動、DASタイマーをリセット
                self.try_move(dir, 0);
                self.das_timer = 0.0;
                self.das_active = false;
                self.last_dir = dir;
            } else {
                self.das_timer += dt;
                // das_active = false の間は初回ディレイ (150ms)、
                // true になってからは ARR (Auto Repeat Rate, 50ms) で連続移動
                let threshold = if self.das_active { 0.05 } else { 0.15 };
                if self.das_timer >= threshold {
                    self.try_move(dir, 0);
                    self.das_timer = 0.0;
                    self.das_active = true;
                }
            }
        } else {
            // キーを離したらすべてリセット
            self.last_dir = 0;
            self.das_active = false;
            self.das_timer = 0.0;
        }

        // ── 自動落下 ──────────────────────────────────────────────────
        // ソフトドロップ中 (↓押下) は落下間隔を 50ms に固定して加速する。
        let interval = if is_key_down(KeyCode::Down) {
            0.05f32
        } else {
            self.fall_interval()
        };

        self.fall_timer += dt;
        if self.fall_timer >= interval {
            self.fall_timer = 0.0;
            if !self.try_move(0, 1) {
                // 着地: ロックディレイタイマーを進める (0.5秒で固定)
                self.lock_timer += interval;
                if self.lock_timer >= 0.5 {
                    self.lock_piece();
                }
            } else {
                // 落下できた → ロックタイマーをリセット
                self.lock_timer = 0.0;
            }
        }
    }

    // is_key_pressed (エッジ検出) を使う操作はここで処理する。
    // update とは分離することで、同フレーム内で重複処理しない。
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
                        27.0,
                        27.0,
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
        draw_text("← → : 移動", lx, BOARD_Y + 60.0, 18.0, GRAY);
        draw_text("↑ / X: 回転", lx, BOARD_Y + 85.0, 18.0, GRAY);
        draw_text("↓ : 落下", lx, BOARD_Y + 110.0, 18.0, GRAY);
        draw_text("SPC: ハードドロップ", lx, BOARD_Y + 135.0, 18.0, GRAY);

        // ── ゲームオーバー画面 ────────────────────────────────────────
        if self.game_over {
            let gw = screen_width();
            let gh = screen_height();
            // 半透明の黒オーバーレイ
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

    // macroquad のメインループ: 毎フレーム next_frame().await でVSyncを待つ。
    loop {
        let dt = get_frame_time(); // 前フレームからの経過時間 (秒)
        clear_background(Color::new(0.05, 0.05, 0.1, 1.0));

        game.handle_keys(); // エッジ入力 (押した瞬間) を処理
        game.update(dt);    // 連続入力・タイマーを処理
        game.draw();

        next_frame().await;
    }
}
