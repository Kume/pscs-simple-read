use pcsc::*;
use serde::{Serialize};
use itertools::Itertools;

macro_rules! debug_println {
    () => {
        if atty::is(atty::Stream::Stdout) {
            println!("\n")
        }
    };
    ($($arg:tt)*) => {{
        if atty::is(atty::Stream::Stdout) {
            println!($($arg)*)
        }
    }};
}

#[derive(Serialize)]
struct CardInfo {
    id: Vec<u8>,
    attr: Vec<u8>,
}

fn main() {
    // let blank_card_attr = vec![59,143,128,1,128,79,12,160,0,0,3,6,3,0,3,0,0,0,0,104];
    let ctx = Context::establish(Scope::User).expect("failed to establish context");
    let mut reader_states: Vec<ReaderState> = vec![];
    
    {
        let mut readers_buf = [0; 2048];
        let names = ctx.list_readers(&mut readers_buf).expect("failed to list readers");
        for name in names {
            reader_states.push(ReaderState::new(name, State::UNAWARE));
        }
    }

    
    loop {
        for rs in &mut reader_states {
            rs.sync_current_state();
        }
        
        ctx.get_status_change(None, &mut reader_states).expect("failed to get status change");
        
        for rs in &reader_states {
            if rs.name() != PNP_NOTIFICATION() {
                debug_println!("{:?} {:?} {:?}", rs.name(), rs.event_state(), rs.atr());
            }
            
            if rs.atr().len() > 0 && rs.event_state().intersects(State::PRESENT) {
                let card = match ctx.connect(rs.name(), ShareMode::Shared, Protocols::T1) {
                    Ok(card) => card,
                    Err(Error::NoSmartcard) => {
                        debug_println!("A smartcard is not present in the reader.");
                        return;
                    }
                    Err(err) => {
                        eprintln!("Failed to connect to card: {}", err);
                        std::process::exit(1);
                    }
                };
                
                debug_println!("connected");
                
                let card_info = {
                    let mut buffer = [0; 256];
                    let command = [0xff, 0xca, 0x00, 0x00, 0x00];
                    let result = card.transmit(&command, &mut buffer);
                    match result {
                        Ok(response) => {
                            debug_println!("command Ok {:?}", response);
                            let result_code = response.get((response.len() - 2)..);
                            let is_success = match result_code {
                                Some(code) => code[0] == 0x90 && code[1] == 0x00,
                                None => false
                            };
                            if is_success {
                                CardInfo {
                                    attr: rs.atr().to_vec(),
                                    id: response.iter().dropping_back(2).cloned().collect(),
                                }
                            } else {
                                eprintln!("Invalid result code: {:?}", result_code);
                                std::process::exit(1);
                            }
                        },
                        Err(err) => {
                            eprintln!("Failed to transmit card: {}", err);
                            std::process::exit(1);
                        }
                    }
                };
                print!("{}", serde_json::to_string(&card_info).unwrap());
                std::process::exit(0);
            }
        }
    }
}
