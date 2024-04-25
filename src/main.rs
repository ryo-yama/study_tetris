//////////////////////////////////////////////////
// Rust  Entity - Component - System 設計を用いたテトリスゲームの作成
//
// @created 2024/01/27
//////////////////////////////////////////////////

//
// Crates
//
use bevy::prelude::*;
use bevy::window::{WindowMode, WindowResolution};
use rand::prelude::*;

//
// Component: Block Props
//
#[derive(Component)]
struct Position {
    x: i32,
    y: i32,
}
#[derive(Component)]
struct Fix;
#[derive(Component)]
struct Free;

#[derive(Component)]
struct RelativePosition {
    rot_x: i32,
    rot_y: i32,
}

//
// Resource: Block
//
#[derive(Resource)]
struct Materials {
    colors: Vec<Color>,
}
#[derive(Resource)]
struct BlockPatterns(Vec<Vec<(i32, i32)>>);

//
// Resource: Timer
//
#[derive(Resource)]
struct GameTimer(Timer);
// 入力を受け付けるタイマー
#[derive(Resource)]
struct InputTimer(Timer);


//
// Resource: GameBoard
//
#[derive(Resource)]
struct GameBoard(Vec<Vec<bool>>);

//
// Event
//
#[derive(Event)]
struct NewBlockEvent;
#[derive(Event)]
struct GameOverEvent;

// １マス当たりのサイズ
const UNIT_WIDTH: u32 = 40;
const UNIT_HEIGHT: u32 = 40;

// 画面全体に表示したいマスのサイズ
const X_LENGTH: u32 = 10;
const Y_LENGTH: u32 = 18;

// 必要な画面サイズ
const SCREEN_WIDTH: u32 = UNIT_WIDTH * X_LENGTH;
const SCREEN_HEIGHT: u32 = UNIT_HEIGHT * Y_LENGTH;

/**
 * メイン関数（エントリーポイント）
 */
fn main() {
    // ウィンドウ設定
    let window_plugin = WindowPlugin {
        primary_window: Some(Window {
            resolution: WindowResolution::new((SCREEN_WIDTH + 5) as f32, (SCREEN_HEIGHT + 5) as f32),
            title: "my tetris".into(),
            mode: WindowMode::Windowed,
            ..Window::default()
        }),
        .. Default::default()
    };

    // アプリ作成
    App::new() 
        .insert_resource(BlockPatterns(vec![
            vec![(0, 0), (0, -1), (0, 1), (0, 2)],  // I
            vec![(0, 0), (0, -1), (0, 1), (-1, 1)], // L
            vec![(0, 0), (0, -1), (0, 1), (1, 1)],  // 逆L
            vec![(0, 0), (0, -1), (1, 0), (1, 1)],  // Z
            vec![(0, 0), (1, 0), (0, 1), (1, -1)],  // 逆Z
            vec![(0, 0), (0, 1), (1, 0), (1, 1)],   // 四角
            vec![(0, 0), (-1, 0), (1, 0), (0, 1)],  // T
        ]))
        .insert_resource(GameTimer(Timer::new(
            std::time::Duration::from_millis(400),
            TimerMode::Repeating,
        )))
        .insert_resource(InputTimer(Timer::new(
            std::time::Duration::from_millis(100),
            TimerMode::Repeating,
        )))
        .insert_resource(GameBoard(vec![vec![false; 25]; 25]))
        .add_plugins(DefaultPlugins.set(window_plugin))
        .add_event::<NewBlockEvent>()
        .add_event::<GameOverEvent>()
        .add_systems(Startup, setup)
        .add_systems(First, delete_line)
        .add_systems(Update, (
                spawn_block,
                position_transform,
                game_timer,
                block_horizontal_move,
                block_vertical_move,
                block_rotate,
                block_fall,
                gameover,
        ))
    .run();
}

/**
 * System: セットアップ
 */
pub(crate) fn setup(mut commands: Commands, mut new_block_events: ResMut<Events<NewBlockEvent>>) {
    // 2D カメラ エンティティの作成
    commands.spawn(Camera2dBundle::default());

    // マテリアルカラーを用意する
    commands.insert_resource(Materials {
        colors: vec![
            Color::rgb(0.25, 0.9, 0.39),
            Color::rgb(0.85, 0.25, 0.35),
            Color::rgb(0.27, 0.59, 0.82),
            Color::rgb(0.89, 0.9, 0.27),
            Color::rgb(0.13, 0.89, 0.94),
            Color::rgb(0.94, 0.55, 0.27),
        ],
    });

    // イベントの送信
    new_block_events.send(NewBlockEvent);
}

/**
 * System: 次のブロックの決定
 */
pub(crate) fn next_block(block_patterns: &Vec<Vec<(i32, i32)>>) -> Vec<(i32, i32)> {
    let mut rng = rand::thread_rng();
    let mut pattern_index: usize = rng.gen();
    pattern_index %= block_patterns.len();

    block_patterns[pattern_index].clone()
}

/**
 * System: ブロックの色の決定
 */
pub(crate) fn next_color(colors: &Vec<Color>) -> Color {
    let mut rng = rand::thread_rng();
    let mut color_index: usize = rng.gen();
    color_index %= colors.len();

    colors[color_index]
}

/**
 * System: ブロックの生成
 */
pub(crate) fn spawn_block(
    mut commands: Commands,
    materials: Res<Materials>,
    block_patterns: Res<BlockPatterns>,
    mut new_block_event_reader: EventReader<NewBlockEvent>,
    game_board: ResMut<GameBoard>,
    mut gameover_events: ResMut<Events<GameOverEvent>>,
) {
    if new_block_event_reader
        .read()
        .next()
        .is_none()
    {
        return;
    }

    let new_block = next_block(&block_patterns.0);
    let new_color = next_color(&materials.colors);

    // ブロックの初期位置
    let initial_x = X_LENGTH / 2;
    let initial_y = Y_LENGTH;// - 4;

    // ゲームオーバー判定
    let gameover = new_block.iter().any(|(r_x, r_y)| { // <--追加
        let pos_x = (initial_x as i32 + r_x) as usize;
        let pos_y = (initial_y as i32 + r_y) as usize;

        game_board.0[pos_y][pos_x]
    });

    if gameover {
        // ブロックを生成せずにゲームオーバーイベントを通知
        gameover_events.send(GameOverEvent);
        println!("Game Over");
        return;
    }

    new_block.iter().for_each(|(r_x, r_y)| {
        // ブロック エンティティの作成
        commands
        .spawn(SpriteBundle {
            sprite: Sprite {
                color: new_color,
                ..Sprite::default()
            },
            ..SpriteBundle::default()
        })
        .insert(Position {
            // ブロックの初期座標
            // x: 0 ～ 9
            // y: 0 ～ 17
            x: (initial_x as i32 + r_x),
            y: (initial_y as i32 + r_y),
        })
        .insert(RelativePosition {
            rot_x: *r_x,
            rot_y: *r_y,
        })
        .insert(Free);
    });
}

/**
 * System: ブロックの移動
 */
pub(crate) fn position_transform(mut position_query: Query<(&Position, &mut Transform, &mut Sprite)>) {
    let origin_x = UNIT_WIDTH as i32 / 2 - SCREEN_WIDTH as i32 / 2;
    let origin_y = UNIT_HEIGHT as i32 / 2 - SCREEN_HEIGHT as i32 / 2;

    position_query
        .iter_mut()
        .for_each(|(pos, mut transform, mut sprite)| {
            transform.translation = Vec3::new(
                (origin_x + pos.x as i32 * UNIT_WIDTH as i32) as f32,
                (origin_y + pos.y as i32 * UNIT_WIDTH as i32) as f32,
                0.0,
            );
            sprite.custom_size = Some(Vec2::new(UNIT_WIDTH as f32, UNIT_HEIGHT as f32))
        });
}

/**
 * System: タイマーを進める
 */
pub(crate) fn game_timer(
    time: Res<Time>,
    mut game_timer: ResMut<GameTimer>,
    mut input_timer: ResMut<InputTimer>
) {
    game_timer.0.tick(time.delta());
    input_timer.0.tick(time.delta());
}

/**
 * System: ブロックの落下
 */
pub(crate) fn block_fall(
    mut commands: Commands,
    timer: ResMut<GameTimer>,
    mut block_query: Query<(Entity, &mut Position, &Free)>,
    mut game_board: ResMut<GameBoard>,
    mut new_block_events: ResMut<Events<NewBlockEvent>>,
) {
    if !timer.0.finished() {
        return;
    }

    // ブロックがそれ以上落下できないかを調べる
    let cannot_fall = block_query.iter_mut().any(|(_, pos, _)| {
        if pos.x as u32 >= X_LENGTH || pos.y as u32 >= Y_LENGTH {
            return false;
        }

        // yが0、または一つ下にブロックがすでに存在する
        pos.y == 0 || game_board.0[(pos.y - 1) as usize][pos.x as usize]
    });

    if cannot_fall {
        // 落下できない
        block_query.iter_mut().for_each(|(entity, pos, _)| {
            commands.entity(entity).remove::<Free>();
            commands.entity(entity).insert(Fix);
            game_board.0[pos.y as usize][pos.x as usize] = true;
        });
        // 新しくブロックを生成するためのイベントを通知
        new_block_events.send(NewBlockEvent);
    } else {
        // 落下
        block_query.iter_mut().for_each(|(_, mut pos, _)| {
            pos.y -= 1;
        });
    }
}

/**
 * System: ブロックの水平移動
 */
pub(crate) fn block_horizontal_move(
    key_input: Res<Input<KeyCode>>,
    timer: ResMut<InputTimer>,
    game_board: ResMut<GameBoard>,
    mut free_block_query: Query<(Entity, &mut Position, &Free)>,
) {
    if !timer.0.finished() {
        return;
    }

    if key_input.pressed(KeyCode::Left) {
        // 左に移動できるか判定
        let ok_move_left = free_block_query.iter_mut().all(|(_, pos, _)| {
            if pos.y as u32 >= Y_LENGTH {
                return pos.x > 0;
            }

            if pos.x == 0 {
                return false;
            }

            !game_board.0[(pos.y) as usize][pos.x as usize - 1]
        });

        if ok_move_left {
            free_block_query.iter_mut().for_each(|(_, mut pos, _)| {
                pos.x -= 1;
            });
        }
    }

    if key_input.pressed(KeyCode::Right) {
        // 右に移動できるか判定
        let ok_move_right = free_block_query.iter_mut().all(|(_, pos, _)| {
            if pos.y as u32 >= Y_LENGTH {
                return pos.x as u32 <= X_LENGTH;
            }

            if pos.x as u32 == X_LENGTH - 1 {
                return false;
            }

            !game_board.0[(pos.y) as usize][pos.x as usize + 1]
        });

        if ok_move_right {
            free_block_query.iter_mut().for_each(|(_, mut pos, _)| {
                pos.x += 1;
            });
        }
    }
}

/**
 * System: ブロックの下移動
 */
pub(crate) fn block_vertical_move(
    key_input: Res<Input<KeyCode>>,
    mut game_board: ResMut<GameBoard>,
    mut free_block_query: Query<(Entity, &mut Position, &Free)>,
) {
    if !key_input.just_pressed(KeyCode::Down) {
        return;
    }

    let mut down_height = 0;
    let mut collide = false;

    // ブロックが衝突する位置を調べる
    while !collide {
        down_height += 1;
        free_block_query.iter_mut().for_each(|(_, pos, _)| {
            if pos.y < down_height {
                collide = true;
                return;
            }

            if game_board.0[(pos.y - down_height) as usize][pos.x as usize] {
                collide = true;
            }
        });
    }

    // ブロックが衝突しないギリギリの位置まで移動
    down_height -= 1;
    free_block_query.iter_mut().for_each(|(_, mut pos, _)| {
        game_board.0[pos.y as usize][pos.x as usize] = false;
        pos.y -= down_height;
        game_board.0[pos.y as usize][pos.x as usize] = true;
    });
}

/**
 * System: ブロックの回転移動
 */
pub(crate) fn block_rotate(
    key_input: Res<Input<KeyCode>>,
    game_board: ResMut<GameBoard>,
    mut free_block_query: Query<(Entity, &mut Position, &mut RelativePosition, &Free)>,
) {
    if !key_input.just_pressed(KeyCode::Up) {
        return;
    }

    // 回転行列を使って新しい絶対座標と相対座標を計算
    fn calc_rotated_pos(pos: &Position, r_pos: &RelativePosition) -> ((i32, i32), (i32, i32)) {
        // cos,-sin,sin,cos (-90)
        let rot_matrix = vec![vec![0, 1], vec![-1, 0]];

        let origin_pos_x = pos.x - r_pos.rot_x;
        let origin_pos_y = pos.y - r_pos.rot_y;

        let new_r_pos_x = rot_matrix[0][0] * r_pos.rot_x + rot_matrix[0][1] * r_pos.rot_y;
        let new_r_pos_y = rot_matrix[1][0] * r_pos.rot_x + rot_matrix[1][1] * r_pos.rot_y;
        let new_pos_x = origin_pos_x + new_r_pos_x;
        let new_pos_y = origin_pos_y + new_r_pos_y;

        ((new_pos_x, new_pos_y), (new_r_pos_x, new_r_pos_y))
    }

    // 回転操作可能かどうか判定
    let rotable = free_block_query.iter_mut().all(|(_, pos, r_pos, _)| {
        let ((new_pos_x, new_pos_y), _) = calc_rotated_pos(&pos, &r_pos);

        let valid_index_x = new_pos_x >= 0 && new_pos_x < X_LENGTH as i32;
        let valid_index_y = new_pos_y >= 0 && new_pos_y < Y_LENGTH as i32;

        if !valid_index_x || !valid_index_y {
            return false;
        }

        !game_board.0[new_pos_y as usize][new_pos_x as usize]
    });

    if !rotable {
        return;
    }

    // 相対座標と絶対座標を更新
    free_block_query
        .iter_mut()
        .for_each(|(_, mut pos, mut r_pos, _)| {
            let ((new_pos_x, new_pos_y), (new_r_pos_x, new_r_pos_y)) =
                calc_rotated_pos(&pos, &r_pos);
            r_pos.rot_x = new_r_pos_x;
            r_pos.rot_y = new_r_pos_y;

            pos.x = new_pos_x;
            pos.y = new_pos_y;
        });
}

/**
 * System: ブロックの削除
 */
pub(crate) fn delete_line(
    mut commands: Commands,
    timer: ResMut<GameTimer>,
    mut game_board: ResMut<GameBoard>,
    mut fixed_block_query: Query<(Entity, &mut Position, &Fix)>,
) {
    if !timer.0.finished() {
        return;
    }

    // 消去対象のブロック行をHashSetに入れていく
    let mut delete_line_set = std::collections::HashSet::new();
    for y in 0..Y_LENGTH {
        let mut delete_current_line = true;
        for x in 0..X_LENGTH {
            if !game_board.0[y as usize][x as usize] {
                delete_current_line = false;
                break;
            }
        }

        if delete_current_line {
            delete_line_set.insert(y);
        }
    }

    // 消去対象ブロック行に含まれるブロックをゲーム盤面から削除する
    fixed_block_query.iter_mut().for_each(|(_, pos, _)| {
        if delete_line_set.get(&(pos.y as u32)).is_some() {
            game_board.0[pos.y as usize][pos.x as usize] = false;
        }
    });

    // 各Y座標について、ブロック消去適用後の新しいY座標を調べる
    let mut new_y = vec![0i32; Y_LENGTH as usize];
    for y in 0..Y_LENGTH {
        let mut down = 0;
        delete_line_set.iter().for_each(|line| {
            if y > *line {
                down += 1;
            }
        });
        new_y[y as usize] = y as i32 - down;
    }


    fixed_block_query.iter_mut().for_each(|(entity, mut pos, _)| {
        if delete_line_set.get(&(pos.y as u32)).is_some() {
            // 消去の対象のブロックをゲームから取り除く
            commands.entity(entity).despawn();
        } else {
            // ブロック消去適用後の新しいY座標を適用
            game_board.0[pos.y as usize][pos.x as usize] = false;
            pos.y = new_y[pos.y as usize];
            game_board.0[pos.y as usize][pos.x as usize] = true;
        }
    });
}

/**
 * System: ゲームオーバー通知を受けた時の処理
 */
pub(crate) fn gameover(
    mut commands: Commands,
    gameover_events: Res<Events<GameOverEvent>>,
    mut game_board: ResMut<GameBoard>,
    mut all_block_query: Query<(Entity, &mut Position)>,
    mut new_block_events: ResMut<Events<NewBlockEvent>>,
) {
    let mut gameover_events_reader = gameover_events.get_reader();

    if gameover_events_reader
        .read(&gameover_events)
        .next()
        .is_none()
    {
        return;
    }

    game_board.0 = vec![vec![false; 25]; 25];
    all_block_query.iter_mut().for_each(|(entity, _)| {
        commands.entity(entity).despawn();
    });

    new_block_events.send(NewBlockEvent);
}