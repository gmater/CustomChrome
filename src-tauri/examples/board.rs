//! Read-only: prints the keyboard options block (command 0x11). Does not write.

fn main() {
    match customchrome_lib::aula::device::read_board() {
        Ok(board) => println!("sleep {} min, 24-hour clock: {}", board.sleep_minutes, board.hour24),
        Err(e) => println!("read failed: {e}"),
    }
}
