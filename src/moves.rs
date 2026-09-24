//! Legal move generation and make-move.

use crate::board::{Board, Castling, Color, Kind, Move, Piece};

fn push_promotions(moves: &mut Vec<Move>, from: usize, to: usize, en_passant: bool) {
    for kind in [Kind::Queen, Kind::Rook, Kind::Bishop, Kind::Knight] {
        moves.push(Move { from, to, promo: Some(kind), en_passant, castle: false });
    }
}

/// Pseudo-legal moves (king may still be left in check).
pub fn pseudo_legal(board: &Board) -> Vec<Move> {
    let mut moves = Vec::with_capacity(48);
    let turn = board.turn;
    for square in 0..64 {
        let Some(piece) = board.squares[square] else { continue };
        if piece.color != turn {
            continue;
        }
        let (x, y) = (square % 8, square / 8);
        match piece.kind {
            Kind::Pawn => gen_pawn(board, square, x, y, turn, &mut moves),
            Kind::Knight => {
                for (dx, dy) in [
                    (1, 2), (2, 1), (2, -1), (1, -2), (-1, -2), (-2, -1), (-2, 1), (-1, 2),
                ] {
                    push_step(board, square, x, y, dx, dy, &mut moves);
                }
            }
            Kind::King => {
                for dy in -1i32..=1 {
                    for dx in -1i32..=1 {
                        if dx != 0 || dy != 0 {
                            push_step(board, square, x, y, dx, dy, &mut moves);
                        }
                    }
                }
                gen_castles(board, &mut moves);
            }
            Kind::Bishop => gen_slides(board, square, x, y, &[(1, 1), (1, -1), (-1, 1), (-1, -1)], &mut moves),
            Kind::Rook => gen_slides(board, square, x, y, &[(1, 0), (-1, 0), (0, 1), (0, -1)], &mut moves),
            Kind::Queen => gen_slides(
                board,
                square,
                x,
                y,
                &[(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (1, -1), (-1, 1), (-1, -1)],
                &mut moves,
            ),
        }
    }
    moves
}

fn push_step(board: &Board, from: usize, x: usize, y: usize, dx: i32, dy: i32, moves: &mut Vec<Move>) {
    let nx = x as i32 + dx;
    let ny = y as i32 + dy;
    if !(0..=7).contains(&nx) || !(0..=7).contains(&ny) {
        return;
    }
    let target = ny as usize * 8 + nx as usize;
    match board.squares[target] {
        None => moves.push(Move { from, to: target, promo: None, en_passant: false, castle: false }),
        Some(piece) if piece.color != board.turn => {
            moves.push(Move { from, to: target, promo: None, en_passant: false, castle: false })
        }
        Some(_) => {}
    }
}

fn gen_slides(board: &Board, from: usize, x: usize, y: usize, dirs: &[(i32, i32)], moves: &mut Vec<Move>) {
    for (dx, dy) in dirs {
        let mut nx = x as i32 + dx;
        let mut ny = y as i32 + dy;
        while (0..=7).contains(&nx) && (0..=7).contains(&ny) {
            let target = ny as usize * 8 + nx as usize;
            match board.squares[target] {
                None => moves.push(Move { from, to: target, promo: None, en_passant: false, castle: false }),
                Some(piece) => {
                    if piece.color != board.turn {
                        moves.push(Move { from, to: target, promo: None, en_passant: false, castle: false });
                    }
                    break;
                }
            }
            nx += dx;
            ny += dy;
        }
    }
}

fn gen_pawn(board: &Board, from: usize, x: usize, y: usize, turn: Color, moves: &mut Vec<Move>) {
    let (dir, start_rank, promo_rank): (i32, usize, usize) = match turn {
        Color::White => (1, 1, 7),
        Color::Black => (-1, 6, 0),
    };
    let one = y as i32 + dir;
    if (0..=7).contains(&one) {
        let one_square = one as usize * 8 + x;
        if board.squares[one_square].is_none() {
            if one as usize == promo_rank {
                push_promotions(moves, from, one_square, false);
            } else {
                moves.push(Move { from, to: one_square, promo: None, en_passant: false, castle: false });
                if y == start_rank {
                    let two = (y as i32 + 2 * dir) as usize * 8 + x;
                    if board.squares[two].is_none() {
                        moves.push(Move { from, to: two, promo: None, en_passant: false, castle: false });
                    }
                }
            }
        }
    }
    for dx in [-1i32, 1] {
        let nx = x as i32 + dx;
        let ny = y as i32 + dir;
        if !(0..=7).contains(&nx) || !(0..=7).contains(&ny) {
            continue;
        }
        let target = ny as usize * 8 + nx as usize;
        // normal capture
        if let Some(piece) = board.squares[target] {
            if piece.color != turn {
                if ny as usize == promo_rank {
                    push_promotions(moves, from, target, false);
                } else {
                    moves.push(Move { from, to: target, promo: None, en_passant: false, castle: false });
                }
            }
        } else if board.en_passant == Some(target) {
            moves.push(Move { from, to: target, promo: None, en_passant: true, castle: false });
        }
    }
}

fn gen_castles(board: &Board, moves: &mut Vec<Move>) {
    let (king_sq, home) = match board.turn {
        Color::White => (4, 0),
        Color::Black => (60, 7),
    };
    // king must not be in check and may not pass through attacked squares
    if board.attacked(king_sq, board.turn.opposite()) {
        return;
    }
    let (king_side, queen_side) = match board.turn {
        Color::White => (board.castling.white_king, board.castling.white_queen),
        Color::Black => (board.castling.black_king, board.castling.black_queen),
    };
    if king_side
        && board.squares[home * 8 + 7] == Some(Piece { color: board.turn, kind: Kind::Rook })
        && board.squares[home * 8 + 5].is_none()
        && board.squares[home * 8 + 6].is_none()
        && !board.attacked(home * 8 + 5, board.turn.opposite())
        && !board.attacked(home * 8 + 6, board.turn.opposite())
    {
        moves.push(Move { from: king_sq, to: home * 8 + 6, promo: None, en_passant: false, castle: true });
    }
    if queen_side
        && board.squares[home * 8] == Some(Piece { color: board.turn, kind: Kind::Rook })
        && board.squares[home * 8 + 1].is_none()
        && board.squares[home * 8 + 2].is_none()
        && board.squares[home * 8 + 3].is_none()
        && !board.attacked(home * 8 + 3, board.turn.opposite())
        && !board.attacked(home * 8 + 2, board.turn.opposite())
    {
        moves.push(Move { from: king_sq, to: home * 8 + 2, promo: None, en_passant: false, castle: true });
    }
}

/// Fully legal moves: pseudo-legal filtered by "own king not attacked
/// afterwards".
pub fn legal_moves(board: &Board) -> Vec<Move> {
    let turn = board.turn;
    pseudo_legal(board)
        .into_iter()
        .filter(|mv| {
            let mut next = board.clone();
            make_move(&mut next, *mv);
            !next.in_check(turn)
        })
        .collect()
}

/// Apply a move (assumed legal); updates castling rights, en passant,
/// clocks.
pub fn make_move(board: &mut Board, mv: Move) {
    let piece = board.squares[mv.from].expect("move from an occupied square");
    let is_pawn = piece.kind == Kind::Pawn;
    let is_capture = board.squares[mv.to].is_some() || mv.en_passant;

    // en-passant capture removes the skipped pawn
    if mv.en_passant {
        let captured = match piece.color {
            Color::White => mv.to - 8,
            Color::Black => mv.to + 8,
        };
        board.squares[captured] = None;
    }

    // castling moves the rook too
    if mv.castle {
        let home = if piece.color == Color::White { 0 } else { 7 };
        match mv.to % 8 {
            6 => {
                board.squares[home * 8 + 5] = board.squares[home * 8 + 7].take();
                board.squares[home * 8 + 7] = None;
            }
            2 => {
                board.squares[home * 8 + 3] = board.squares[home * 8].take();
                board.squares[home * 8] = None;
            }
            _ => {}
        }
    }

    board.squares[mv.to] = match mv.promo {
        Some(kind) => Some(Piece { color: piece.color, kind }),
        None => Some(piece),
    };
    board.squares[mv.from] = None;

    // double pawn push sets the en-passant target
    board.en_passant = if is_pawn && (mv.to as i32 - mv.from as i32).abs() == 16 {
        Some((mv.from + mv.to) / 2)
    } else {
        None
    };

    // castling rights: king moves kill both; rook moves/captures kill one
    update_castling(board, mv);
    if piece.kind == Kind::King {
        match piece.color {
            Color::White => {
                board.castling.white_king = false;
                board.castling.white_queen = false;
            }
            Color::Black => {
                board.castling.black_king = false;
                board.castling.black_queen = false;
            }
        }
    }

    // reset the 50-move counter on pawn moves and captures
    if is_pawn || is_capture {
        board.halfmove = 0;
    } else {
        board.halfmove += 1;
    }

    if board.turn == Color::Black {
        board.fullmove += 1;
    }
    board.turn = board.turn.opposite();
}

fn update_castling(board: &mut Board, mv: Move) {
    // a rook leaving (or being captured on) a home square kills that right
    type CornerClear = (usize, fn(&mut Castling));
    let corners: [CornerClear; 4] = [
        (0, |c: &mut Castling| c.white_queen = false),
        (7, |c: &mut Castling| c.white_king = false),
        (56, |c: &mut Castling| c.black_queen = false),
        (63, |c: &mut Castling| c.black_king = false),
    ];
    for (square, clear) in corners {
        if mv.from == square || mv.to == square {
            clear(&mut board.castling);
        }
    }
}
