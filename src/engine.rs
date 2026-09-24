//! Perft, evaluation and alpha-beta search, SAN/PGN generation.

use crate::board::{Board, Color, Kind, Move, Piece};
use crate::moves::{legal_moves, make_move};

/// Perft: count leaf nodes at `depth`. The correctness gate for move
/// generation — known values pin every rule (castling, en passant,
/// promotions, pins, checks).
pub fn perft(board: &Board, depth: usize) -> u64 {
    if depth == 0 {
        return 1;
    }
    let moves = legal_moves(board);
    if depth == 1 {
        return moves.len() as u64;
    }
    let mut total = 0u64;
    for mv in moves {
        let mut next = board.clone();
        make_move(&mut next, mv);
        total += perft(&next, depth - 1);
    }
    total
}

/// Material values in centipawns.
fn material_value(kind: Kind) -> i32 {
    match kind {
        Kind::Pawn => 100,
        Kind::Knight => 320,
        Kind::Bishop => 330,
        Kind::Rook => 500,
        Kind::Queen => 900,
        Kind::King => 0,
    }
}

/// Simple piece-square preference: center is worth more, pawns advance.
fn positional_bonus(kind: Kind, square: usize, color: Color) -> i32 {
    let (x, y) = (square % 8, square / 8);
    let center = 7 - ((x as i32 - 3).abs() + (y as i32 - 3).abs());
    let advance = match color {
        Color::White => y as i32,        // larger y = deeper into black territory
        Color::Black => 7 - y as i32,
    };
    match kind {
        Kind::Pawn => advance * 6,
        Kind::Knight => center * 4,
        Kind::Bishop => center * 3,
        Kind::Rook => 0,
        Kind::Queen => center,
        Kind::King => -center, // stay out of the middle early on
    }
}

/// Static evaluation from White's point of view, in centipawns.
pub fn evaluate(board: &Board) -> i32 {
    let mut score = 0;
    for square in 0..64 {
        if let Some(piece) = board.squares[square] {
            let value = material_value(piece.kind) + positional_bonus(piece.kind, square, piece.color);
            score += match piece.color {
                Color::White => value,
                Color::Black => -value,
            };
        }
    }
    score
}

const QUIESCENCE_MAX_DEPTH: usize = 6;

fn quiescence(board: &Board, mut alpha: i32, beta: i32, depth: usize) -> i32 {
    let stand = evaluate(board) * side_sign(board);
    if stand >= beta || depth == 0 {
        return if stand >= beta { beta } else { alpha };
    }
    if stand > alpha {
        alpha = stand;
    }
    // captures only, bounded depth — unbounded capture chains explode
    for mv in legal_moves(board) {
        if board.squares[mv.to].is_none() && !mv.en_passant {
            continue;
        }
        let mut next = board.clone();
        make_move(&mut next, mv);
        if next.in_check(board.turn) {
            continue;
        }
        let score = quiescence(&next, alpha, beta, depth - 1);
        if score >= beta {
            return beta;
        }
        if score > alpha {
            alpha = score;
        }
    }
    alpha
}

fn side_sign(board: &Board) -> i32 {
    match board.turn {
        Color::White => 1,
        Color::Black => -1,
    }
}

/// Negamax with alpha-beta pruning. Returns the score from the side to
/// move's point of view.
fn negamax(board: &Board, depth: usize, mut alpha: i32, beta: i32) -> i32 {
    if depth == 0 {
        return quiescence(board, alpha, beta, QUIESCENCE_MAX_DEPTH);
    }
    let moves = legal_moves(board);
    if moves.is_empty() {
        return if board.in_check(board.turn) {
            -100_000 - depth as i32 // checkmate: worse when sooner
        } else {
            0 // stalemate
        };
    }
    // order captures first (cheap move ordering helps pruning a lot)
    let mut ordered: Vec<(Move, i32)> = moves
        .into_iter()
        .map(|mv| {
            let victim = board.squares[mv.to].map(|p: Piece| material_value(p.kind)).unwrap_or(0);
            (mv, victim)
        })
        .collect();
    ordered.sort_by_key(|pair| std::cmp::Reverse(pair.1));

    let mut best = i32::MIN;
    for (mv, _) in ordered {
        let mut next = board.clone();
        make_move(&mut next, mv);
        let score = -negamax(&next, depth - 1, -beta, -alpha);
        if score > best {
            best = score;
        }
        if best > alpha {
            alpha = best;
        }
        if alpha >= beta {
            break;
        }
    }
    best
}

/// Search the position and return the best move.
pub fn best_move(board: &Board, depth: usize) -> Option<Move> {
    let moves = legal_moves(board);
    let mut best: Option<(Move, i32)> = None;
    let mut ordered: Vec<(Move, i32)> = moves
        .into_iter()
        .map(|mv| {
            let victim = board.squares[mv.to].map(|p: Piece| material_value(p.kind)).unwrap_or(0);
            (mv, victim)
        })
        .collect();
    ordered.sort_by_key(|pair| std::cmp::Reverse(pair.1));
    for (mv, _) in ordered {
        let mut next = board.clone();
        make_move(&mut next, mv);
        let score = -negamax(&next, depth.saturating_sub(1), -i32::MAX, i32::MAX);
        if best.is_none() || score > best.unwrap().1 {
            best = Some((mv, score));
        }
    }
    best.map(|(mv, _)| mv)
}

/// Standard algebraic notation for a move in the given position (the
/// position BEFORE the move). Includes disambiguation and castling.
pub fn to_san(board: &Board, mv: &Move) -> String {
    if mv.castle {
        return if mv.to % 8 == 6 { "O-O".into() } else { "O-O-O".into() };
    }
    let Some(piece) = board.squares[mv.from] else { return "?".into() };
    let is_capture = board.squares[mv.to].is_some() || mv.en_passant;

    let mut san = String::new();
    if piece.kind == Kind::Pawn {
        if is_capture {
            san.push((b'a' + (mv.from % 8) as u8) as char);
        }
        san.push_str(&square_name(mv.to));
        if let Some(kind) = mv.promo {
            san.push('=');
            san.push(kind_letter(kind));
        }
    } else {
        san.push(kind_letter(piece.kind));
        // disambiguation: another same-kind piece can also reach `to`?
        let ambiguous: Vec<usize> = legal_moves(board)
            .into_iter()
            .filter(|other| {
                other.to == mv.to
                    && other.from != mv.from
                    && board.squares[other.from].map(|p| p.kind) == Some(piece.kind)
            })
            .map(|other| other.from)
            .collect();
        if !ambiguous.is_empty() {
            let same_file = ambiguous.iter().any(|sq| sq % 8 == mv.from % 8);
            let same_rank = ambiguous.iter().any(|sq| sq / 8 == mv.from / 8);
            if !same_file {
                san.push((b'a' + (mv.from % 8) as u8) as char);
            } else if !same_rank {
                san.push((b'1' + (mv.from / 8) as u8) as char);
            } else {
                san.push_str(&square_name(mv.from));
            }
        }
        if is_capture {
            san.push('x');
        }
        san.push_str(&square_name(mv.to));
    }

    // check / mate suffix
    let mut next = board.clone();
    make_move(&mut next, *mv);
    if next.in_check(next.turn) {
        san.push(if legal_moves(&next).is_empty() { '#' } else { '+' });
    }
    san
}

fn kind_letter(kind: Kind) -> char {
    match kind {
        Kind::Pawn => ' ',
        Kind::Knight => 'N',
        Kind::Bishop => 'B',
        Kind::Rook => 'R',
        Kind::Queen => 'Q',
        Kind::King => 'K',
    }
}

fn square_name(square: usize) -> String {
    format!(
        "{}{}",
        (b'a' + (square % 8) as u8) as char,
        (b'1' + (square / 8) as u8) as char
    )
}

/// Play a full engine-vs-engine game; returns the SAN move list, the FEN
/// after every move, and the material evolution (white, black).
pub fn self_play(board: &Board, depth: usize, max_plies: usize) -> SelfPlayGame {
    let mut board = board.clone();
    let mut sans: Vec<String> = Vec::new();
    let mut fens: Vec<String> = vec![board.to_fen()];
    let mut material: Vec<(i32, i32)> = vec![material_pair(&board)];

    for _ply in 0..max_plies {
        if board.halfmove >= 100 {
            break; // fifty-move rule
        }
        let Some(mv) = best_move(&board, depth) else { break };
        let san = to_san(&board, &mv);
        make_move(&mut board, mv);
        sans.push(san);
        fens.push(board.to_fen());
        material.push(material_pair(&board));
        if board.in_check(board.turn) && legal_moves(&board).is_empty() {
            break; // checkmate
        }
    }
    SelfPlayGame { sans, fens, material }
}

pub struct SelfPlayGame {
    pub sans: Vec<String>,
    pub fens: Vec<String>,
    pub material: Vec<(i32, i32)>,
}

fn material_pair(board: &Board) -> (i32, i32) {
    let mut white = 0;
    let mut black = 0;
    for piece in board.squares.iter().flatten() {
        let value = material_value(piece.kind);
        match piece.color {
            Color::White => white += value,
            Color::Black => black += value,
        }
    }
    (white, black)
}

/// Divide: perft split by root move (debugging helper).
pub fn divide(board: &Board, depth: usize) -> Vec<(String, u64)> {
    let mut out = Vec::new();
    for mv in legal_moves(board) {
        let mut next = board.clone();
        make_move(&mut next, mv);
        out.push((to_san(board, &mv), perft(&next, depth - 1)));
    }
    out.sort();
    out
}
