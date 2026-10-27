//! Uses the app's own device code. `setup`: puts macro 1 on numpad / (matrix 30), only if
//! that key is at its factory setting and a macro exists. `restore`: puts numpad / back and
//! deletes macro 1, only if numpad / still plays it.

use customchrome_lib::aula::device;

const KEY: u8 = 30;
const PLAY_ONCE: [u8; 4] = [6, 0, 0, 0];

fn main() {
    let mode = std::env::args().nth(1).unwrap_or_default();
    let state = device::read_macros().expect("read macros");
    let key = state.keys[KEY as usize];
    println!("macros on keyboard: {:?}", state.macros.iter().map(|m| m.len()).collect::<Vec<_>>());
    match mode.as_str() {
        "setup" => {
            if state.macros.is_empty() || key != [2, 0, 0x54, 0] {
                println!("not in the expected state (numpad / = {key:02X?}); leaving it alone");
                return;
            }
            let keys = device::assign_key(KEY, Some(PLAY_ONCE)).expect("assign");
            println!("numpad / now {:02X?}", keys[KEY as usize]);
        }
        "restore" => {
            if key != PLAY_ONCE {
                println!("numpad / no longer plays the test macro ({key:02X?}); leaving it alone");
                return;
            }
            let after = device::delete_macro(0).expect("delete");
            println!("numpad / now {:02X?}, macros left: {}", after.keys[KEY as usize], after.macros.len());
        }
        _ => println!("usage: macro_test setup|restore"),
    }
}
