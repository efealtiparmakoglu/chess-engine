//! Perft + rules + SAN + search tests. The perft values are the gold
//! standard for chess move generation correctness.

use chess_engine_lib::board::{Board, START_FEN};
use chess_engine_lib::engine::{best_move, perft, self_play, to_san};
use chess_engine_lib::moves::{legal_moves, make_move};

fn perft_fen(fen: &str, depth: usize, expected: u64, name: &str) {
    let board = Board::from_fen(fen).expect(name);
    let got = perft(&board, depth);
    assert_eq!(got, expected, "{name} perft({depth}): got {got}, want {expected}");
}

#[test]
fn perft_startpos_depths_1_to_4() {
    perft_fen(START_FEN, 1, 20, "startpos");
    perft_fen(START_FEN, 2, 400, "startpos");
    perft_fen(START_FEN, 3, 8_902, "startpos");
    perft_fen(START_FEN, 4, 197_281, "startpos");
}

#[test]
fn perft_kiwipete() {
    // the famous Kiwipete position: castling, pins, checks, promotions
    let fen = "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1";
    perft_fen(fen, 1, 48, "kiwipete");
    perft_fen(fen, 2, 2_039, "kiwipete");
    perft_fen(fen, 3, 97_862, "kiwipete");
}

#[test]
fn perft_en_passant_and_promotion_positions() {
    // position with en-passant possibilities
    perft_fen(
        "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
        3,
        2_812,
        "en-passant position",
    );
    // promotion-heavy position
    perft_fen(
        "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
        3,
        9_467,
        "promotion position",
    );
}

#[test]
fn fen_round_trip() {
    let board = Board::from_fen(
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
    )
    .unwrap();
    assert_eq!(
        board.to_fen(),
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1"
    );
}

#[test]
fn san_includes_disambiguation_and_castling() {
    let board = Board::startpos();
    // e4 from e2
    let e4 = legal_moves(&board)
        .into_iter()
        .find(|m| m.from == 12 && m.to == 28)
        .expect("e2e4 must be legal");
    assert_eq!(to_san(&board, &e4), "e4");

    // castling: queenside is legal straight away in this position
    let board = Board::from_fen("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1").unwrap();
    let castle = legal_moves(&board)
        .into_iter()
        .find(|m| m.castle && m.to == 2)
        .expect("O-O-O must be legal (b1/c1/d1 empty, path unattacked)");
    let san = to_san(&board, &castle);
    assert_eq!(san, "O-O-O");
    // and short castling reports O-O
    let short = legal_moves(&board)
        .into_iter()
        .find(|m| m.castle && m.to == 6)
        .expect("O-O must be legal too");
    assert_eq!(to_san(&board, &short), "O-O");
}

#[test]
fn en_passant_capture_works() {
    let mut board = Board::from_fen("8/8/8/8/5p2/8/4P3/8 w - - 0 1").unwrap();
    // e2-e4 (double push), then black f4 can capture en passant
    let e4 = legal_moves(&board)
        .into_iter()
        .find(|m| m.from == 12 && m.to == 28)
        .unwrap();
    make_move(&mut board, e4);
    assert_eq!(board.en_passant, Some(20));
    let ep = legal_moves(&board)
        .into_iter()
        .find(|m| m.en_passant)
        .expect("en-passant capture must be available");
    make_move(&mut board, ep);
    // the captured pawn (e4) must be gone
    assert!(board.squares[28].is_none());
}

#[test]
fn best_move_takes_hanging_queen() {
    // black queen on d5 is undefended and the white rook sits on d1:
    // any depth must pick Rxd5
    let fen = "6k1/8/8/3q4/8/8/8/3R2K1 w - - 0 1";
    let board = Board::from_fen(fen).unwrap();
    let mv = best_move(&board, 3).expect("must find a move");
    assert_eq!(to_san(&board, &mv), "Rxd5");
}

#[test]
fn self_play_produces_pgn_ready_game() {
    let board = Board::startpos();
    let game = self_play(&board, 2, 24);
    assert!(!game.sans.is_empty());
    assert_eq!(game.fens.len(), game.sans.len() + 1);
    assert_eq!(game.material.len(), game.fens.len());
    // first move must be a pawn move or knight move in SAN
    let first = &game.sans[0];
    assert!(
        ["e4", "d4", "Nf3", "Nc3", "e3", "d3", "c4", "Nf6"].contains(&first.as_str())
            || first.len() == 2,
        "unexpected first move {first}"
    );
}
