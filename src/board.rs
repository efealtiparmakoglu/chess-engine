//! Board representation, FEN parsing and attack detection.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Color {
    White,
    Black,
}

impl Color {
    pub fn opposite(self) -> Color {
        match self {
            Color::White => Color::Black,
            Color::Black => Color::White,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Piece {
    pub color: Color,
    pub kind: Kind,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Castling {
    pub white_king: bool,
    pub white_queen: bool,
    pub black_king: bool,
    pub black_queen: bool,
}

/// One move: from-square, to-square, promotion kind (None except pawn
/// promotions), and flags for castling/en-passant bookkeeping.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Move {
    pub from: usize, // 0..63, a1 = 0, h1 = 7, a8 = 56
    pub to: usize,
    pub promo: Option<Kind>,
    pub en_passant: bool,
    pub castle: bool,
}

#[derive(Clone)]
pub struct Board {
    pub squares: [Option<Piece>; 64],
    pub turn: Color,
    pub castling: Castling,
    /// en-passant target square (the skipped pawn square), if any
    pub en_passant: Option<usize>,
    pub halfmove: u32,
    pub fullmove: u32,
}

pub const START_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

fn kind_from_char(c: char) -> Option<(Color, Kind)> {
    let color = if c.is_ascii_uppercase() { Color::White } else { Color::Black };
    let kind = match c.to_ascii_lowercase() {
        'p' => Kind::Pawn,
        'n' => Kind::Knight,
        'b' => Kind::Bishop,
        'r' => Kind::Rook,
        'q' => Kind::Queen,
        'k' => Kind::King,
        _ => return None,
    };
    Some((color, kind))
}

fn kind_to_char(kind: Kind, color: Color) -> char {
    let c = match kind {
        Kind::Pawn => 'p',
        Kind::Knight => 'n',
        Kind::Bishop => 'b',
        Kind::Rook => 'r',
        Kind::Queen => 'q',
        Kind::King => 'k',
    };
    if color == Color::White {
        c.to_ascii_uppercase()
    } else {
        c
    }
}

impl Board {
    pub fn from_fen(fen: &str) -> Result<Board, String> {
        let mut board = Board {
            squares: [None; 64],
            turn: Color::White,
            castling: Castling {
                white_king: false,
                white_queen: false,
                black_king: false,
                black_queen: false,
            },
            en_passant: None,
            halfmove: 0,
            fullmove: 1,
        };
        let parts: Vec<&str> = fen.split_whitespace().collect();
        if parts.len() < 4 {
            return Err("fen needs at least 4 fields".into());
        }
        // FEN lists rank 8 first; squares[0] is a1
        let mut rank = 7usize;
        let mut file = 0usize;
        for c in parts[0].chars() {
            match c {
                '/' => {
                    rank -= 1;
                    file = 0;
                }
                digit if digit.is_ascii_digit() => {
                    file += digit.to_digit(10).unwrap() as usize;
                }
                piece_char => {
                    if rank > 7 || file > 7 {
                        return Err("fen board overflows".into());
                    }
                    if let Some((color, kind)) = kind_from_char(piece_char) {
                        board.squares[rank * 8 + file] = Some(Piece { color, kind });
                        file += 1;
                    } else {
                        return Err(format!("bad fen char {c}"));
                    }
                }
            }
        }
        if rank != 0 || file != 8 {
            return Err(format!("fen board ends at rank {rank} file {file}, expected rank 0 file 8"));
        }
        board.turn = match parts[1] {
            "w" => Color::White,
            "b" => Color::Black,
            _ => return Err("bad turn field".into()),
        };
        for c in parts[2].chars() {
            match c {
                'K' => board.castling.white_king = true,
                'Q' => board.castling.white_queen = true,
                'k' => board.castling.black_king = true,
                'q' => board.castling.black_queen = true,
                '-' => {}
                _ => return Err("bad castling field".into()),
            }
        }
        if parts[3] != "-" {
            let bytes = parts[3].as_bytes();
            let file = (bytes[0] - b'a') as usize;
            let rank = (bytes[1] - b'1') as usize;
            board.en_passant = Some(rank * 8 + file);
        }
        if let Ok(halfmove) = parts[4].parse() {
            board.halfmove = halfmove;
        }
        if let Ok(fullmove) = parts.get(5).map(|s| s.parse()).unwrap_or(Ok(1)) {
            board.fullmove = fullmove;
        }
        Ok(board)
    }

    pub fn startpos() -> Board {
        Board::from_fen(START_FEN).expect("start position fen is valid")
    }

    pub fn to_fen(&self) -> String {
        let mut fen = String::new();
        for rank in (0..8).rev() {
            let mut empty = 0;
            for file in 0..8 {
                match self.squares[rank * 8 + file] {
                    Some(piece) => {
                        if empty > 0 {
                            fen.push_str(&empty.to_string());
                            empty = 0;
                        }
                        fen.push(kind_to_char(piece.kind, piece.color));
                    }
                    None => empty += 1,
                }
            }
            if empty > 0 {
                fen.push_str(&empty.to_string());
            }
            if rank > 0 {
                fen.push('/');
            }
        }
        fen.push(' ');
        fen.push(if self.turn == Color::White { 'w' } else { 'b' });
        fen.push(' ');
        let mut castling = String::new();
        if self.castling.white_king { castling.push('K'); }
        if self.castling.white_queen { castling.push('Q'); }
        if self.castling.black_king { castling.push('k'); }
        if self.castling.black_queen { castling.push('q'); }
        fen.push_str(if castling.is_empty() { "-" } else { &castling });
        fen.push(' ');
        match self.en_passant {
            Some(sq) => {
                fen.push((b'a' + (sq % 8) as u8) as char);
                fen.push((b'1' + (sq / 8) as u8) as char);
            }
            None => fen.push('-'),
        }
        fen.push_str(&format!(" {} {}", self.halfmove, self.fullmove));
        fen
    }

    pub fn king_square(&self, color: Color) -> Option<usize> {
        self.squares.iter().position(|s| {
            matches!(s, Some(piece) if piece.kind == Kind::King && piece.color == color)
        })
    }

    /// Is `square` attacked by any piece of `by`?
    pub fn attacked(&self, square: usize, by: Color) -> bool {
        let (sq_x, sq_y) = (square % 8, square / 8);
        // pawns: white advances toward larger rows, black toward smaller,
        // so a pawn attacking `square` sits one row BEHIND its direction
        let pawn_row = sq_y as i32 - if by == Color::White { 1 } else { -1 };
        if (0..8).contains(&pawn_row) {
            for dx in [-1i32, 1] {
                let x = sq_x as i32 + dx;
                if (0..8).contains(&x) {
                    if let Some(piece) = self.squares[pawn_row as usize * 8 + x as usize] {
                        if piece.color == by && piece.kind == Kind::Pawn {
                            return true;
                        }
                    }
                }
            }
        }
        // knights
        for (dx, dy) in [(1, 2), (2, 1), (2, -1), (1, -2), (-1, -2), (-2, -1), (-2, 1), (-1, 2)] {
            let x = sq_x as i32 + dx;
            let y = sq_y as i32 + dy;
            if (0..8).contains(&x) && (0..8).contains(&y) {
                if let Some(piece) = self.squares[y as usize * 8 + x as usize] {
                    if piece.color == by && piece.kind == Kind::Knight {
                        return true;
                    }
                }
            }
        }
        // king
        for dy in -1i32..=1 {
            for dx in -1i32..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let x = sq_x as i32 + dx;
                let y = sq_y as i32 + dy;
                if (0..8).contains(&x) && (0..8).contains(&y) {
                    if let Some(piece) = self.squares[y as usize * 8 + x as usize] {
                        if piece.color == by && piece.kind == Kind::King {
                            return true;
                        }
                    }
                }
            }
        }
        // sliders: rook/queen along ranks and files
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let mut x = sq_x as i32 + dx;
            let mut y = sq_y as i32 + dy;
            while (0..8).contains(&x) && (0..8).contains(&y) {
                if let Some(piece) = self.squares[y as usize * 8 + x as usize] {
                    if piece.color == by
                        && matches!(piece.kind, Kind::Rook | Kind::Queen)
                    {
                        return true;
                    }
                    break;
                }
                x += dx;
                y += dy;
            }
        }
        // sliders: bishop/queen along diagonals
        for (dx, dy) in [(1, 1), (1, -1), (-1, 1), (-1, -1)] {
            let mut x = sq_x as i32 + dx;
            let mut y = sq_y as i32 + dy;
            while (0..8).contains(&x) && (0..8).contains(&y) {
                if let Some(piece) = self.squares[y as usize * 8 + x as usize] {
                    if piece.color == by
                        && matches!(piece.kind, Kind::Bishop | Kind::Queen)
                    {
                        return true;
                    }
                    break;
                }
                x += dx;
                y += dy;
            }
        }
        false
    }

    pub fn in_check(&self, color: Color) -> bool {
        self.king_square(color)
            .map(|sq| self.attacked(sq, color.opposite()))
            .unwrap_or(false)
    }
}
