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
struct GeneralCardInfo {
    id: Vec<u8>,
    attr: Vec<u8>,
}

#[derive(Serialize)]
struct FelicaCardInfo {
    id: Vec<u8>,
    attr: Vec<u8>,
    amount: i32,
    emoneyType: String,
}

#[derive(Debug)]
enum FelicaSystemCodeType {
    TranspotationCard,
    Others,
}

#[derive(Serialize)]
#[serde(tag = "type")]
enum CardInfo {
    #[serde(rename = "general")]
    General(GeneralCardInfo),
    #[serde(rename = "falica")]
    Felica(FelicaCardInfo),
}

const FELICA_ATTR: [u8; 20] = [59,143,128,1,128,79,12,160,0,0,3,6,17,0,59,0,0,0,0,66];

fn u8_array_equals(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        debug_println!("len not equals {} {}", a.len(), b.len());
        return false
    }
    for (ia, ib) in a.iter().zip(b.iter()) {
        if ia != ib {
            debug_println!("value not equals {} {}", ia, ib);
            return false
        }
    }
    true
}

fn felica_polling(card: &Card) -> (FelicaSystemCodeType, Vec<u8>) {
    let mut receive_buffer = [0; 256];
    let command_for_transportation = [0xFF, 0xAB, 0x00, 0x00, 0x05, 0x00, 0x00, 0x03, 0x01, 0x00];
    let command_for_others = [0xFF, 0xAB, 0x00, 0x00, 0x05, 0x00, 0xFE, 0x00, 0x01, 0x00];
    for _ in 0..5 {
        {
            let result = card.transmit(&command_for_transportation, &mut receive_buffer)
                .expect("felica polling for transpotation card failed");
            debug_println!("felica polling result. {:?}", result);
            if result.len() == 22 {
                return (FelicaSystemCodeType::TranspotationCard, result.iter().skip(2).take(8).cloned().collect());
            }
        }
        {
            let result = card.transmit(&command_for_transportation, &mut receive_buffer)
                .expect("felica polling for transpotation card failed");
            debug_println!("felica polling result. {:?}", result);
            if result.len() == 22 {
                return (FelicaSystemCodeType::TranspotationCard, result.iter().skip(2).take(8).cloned().collect());
            }
        }
        {
            let result = card.transmit(&command_for_others, &mut receive_buffer).expect("felica polling for others failed");
            debug_println!("felica polling result. {:?}", result);
            if result.len() == 22 {
                return (FelicaSystemCodeType::Others, result.iter().skip(2).take(8).cloned().collect());
            }
        }
    }
    panic!("poling failed.");
}

fn felica_read_command(id: &Vec<u8>, system_code: &Vec<u8>) -> Vec<u8> {
    let felica_command = itertools::concat([vec![0x06], id.clone(), system_code.clone(), vec![1, 0x80, 0x00]]);
    let command_heder = vec![0xFF, 0xAB, 0x00, 0x00, felica_command.len().try_into().unwrap()];
    itertools::concat([command_heder, felica_command])
}

fn read_block_result_is_valid(result: &[u8], name: &str) -> bool {
    debug_println!("read {} amount result. {:?} {}", name, result, result.len());
    return result.len() == 31 && result[29] == 0x90 && result[30] == 0;
}

fn read_felica_card_info(attr: Vec<u8>, card: &Card) -> CardInfo {
    let suica_amount_service_code: Vec<u8> = vec![1, 0x0F, 0x09];
    let waon_amount_service_code: Vec<u8> = vec![1, 0x17, 0x68];
    let nanaco_amount_service_code: Vec<u8> = vec![1, 0x97, 0x55];
    let edy_amount_service_code: Vec<u8> = vec![1, 0x0F, 0x17];

    let mut receive_buffer = [0; 256];
    let (system_type, id) = felica_polling(card);
    debug_println!("polling result = {:?} {:?}", system_type, id);

    
    for _ in 0..10 {
        {
            let command = felica_read_command(&id, &suica_amount_service_code);
            let result = card.transmit(&command, &mut receive_buffer).expect("read suica amount failed");
            if read_block_result_is_valid(result, "suica") {
                let amount = i32::from(result[13 + 0xF]) + i32::from(result[13 + 0xE]) * 256;
                debug_println!("amount ={}", amount);
                return CardInfo::Felica(FelicaCardInfo {id, attr, amount, emoneyType: String::from("suica")})
            }
        }

        {
            let command = felica_read_command(&id, &edy_amount_service_code);
            let result = card.transmit(&command, &mut receive_buffer).expect("read edy amount failed");
            if read_block_result_is_valid(result, "edy") {
                let amount = i32::from(result[13 + 0xF]) + i32::from(result[13 + 0xE]) * 256;
                debug_println!("amount ={}", amount);
                return CardInfo::Felica(FelicaCardInfo {id, attr, amount, emoneyType: String::from("edy")})
            }
        }
        
        {
            let command = felica_read_command(&id, &nanaco_amount_service_code);
            let result = card.transmit(&command, &mut receive_buffer).expect("read nanaco amount failed");
            if read_block_result_is_valid(result, "nanaco") {
                let amount = i32::from(result[13 + 0x0]) + i32::from(result[13 + 0x1]) * 256;
                debug_println!("amount ={}", amount);
                return CardInfo::Felica(FelicaCardInfo {id, attr, amount, emoneyType: String::from("nanaco")})
            }
        }
        
        {
            let command = felica_read_command(&id, &waon_amount_service_code);
            let result = card.transmit(&command, &mut receive_buffer).expect("read waon amount failed");
            if read_block_result_is_valid(result, "waon") {
                let amount = i32::from(result[13 + 0x0]) + i32::from(result[13 + 0x1]) * 256;
                debug_println!("amount ={}", amount);
                return CardInfo::Felica(FelicaCardInfo {id, attr, amount, emoneyType: String::from("waon")})
            }
        }
    }

    // match system_type {
    //     FelicaSystemCodeType::TranspotationCard => {
    //         {
    //             let command = felica_read_command(&id, &suica_amount_service_code);
    //             for _ in 0..10 {
    //                 let result = card.transmit(&command, &mut receive_buffer).expect("read suica amount failed");
    //                 if read_block_result_is_valid(result, "suica") {
    //                     let amount = i32::from(result[13 + 0xA]) + i32::from(result[13 + 0xB]) * 256;
    //                     debug_println!("amount ={}", amount);
    //                     return CardInfo::Felica(FelicaCardInfo {id, attr, amount, emoneyType: String::from("suica")})
    //                 }
    //             }
    //         }
    //     },
    //     FelicaSystemCodeType::Others => {
    //     }
    // }
    // {
    //     // let command = vec!(0xFF, 0xAB, 0x00, 0x00);
    //     let felica_command = itertools::concat([vec![0x06], id.clone(), vec![1, 0x0F, 0x09], vec![1, 0x80, 0x00]]);
    //     let command_heder = vec![0xFF, 0xAB, 0x00, 0x00, felica_command.len().try_into().unwrap()];
    //     let command = itertools::concat([command_heder, felica_command]);
    //     for _ in 0..10 {
    //         let result = card.transmit(&command, &mut receive_buffer).expect("read suica amount failed");
    //         debug_println!("read suica amount result. {:?} {}", result, result.len());
    //         if result.len() == 31 && result[29] == 0x90 && result[30] == 0 {
    //             let amount = i32::from(result[23]) + i32::from(result[24]) * 256;
    //             debug_println!("amount ={}", amount);
    //             return CardInfo::Felica(FelicaCardInfo {id, attr, amount})
    //         }
    //     }
    // }
    panic!("read_felica_card_info failed.");
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
                    if u8_array_equals(&rs.atr(), &FELICA_ATTR) {
                        read_felica_card_info(rs.atr().to_vec(), &card)
                    } else {
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
                                let id: Vec<u8> = response.iter().dropping_back(2).cloned().collect();
                                if is_success {
                                    CardInfo::General(GeneralCardInfo {
                                        attr: rs.atr().to_vec(),
                                        id,
                                    })
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
                    }
                };
                print!("{}", serde_json::to_string(&card_info).unwrap());
                std::process::exit(0);
            } else if rs.event_state().intersects(State::EMPTY) {
                // debug_println!("try connect to reader {:?}", rs.name());
                // let card = match ctx.connect(rs.name(), ShareMode::Direct, Protocols::empty()){
                //     Ok(card) => card,
                //     Err(Error::NoSmartcard) => {
                //         debug_println!("Unexpected error.");
                //         return;
                //     }
                //     Err(err) => {
                //         eprintln!("Failed to connect to reader: {}", err);
                //         std::process::exit(1);
                //     }
                // };
                
                // debug_println!("connected");

                // {
                //     let mut buffer = [0; 256];
                //     let command = [0xFF, 0xAB, 0x00, 0x00, 0x05, 0x00, 0xFF, 0xFF, 0x01, 0x00];
                //     let result = card.control(ctl_code(3500), &command, &mut buffer);
                //     match result {
                //         Ok(response) => {
                //             debug_println!("command Ok {:?}", response);
                //         },
                //         Err(err) => {
                //             eprintln!("Failed to transmit reader: {}", err);
                //             std::process::exit(1);
                //         }
                //     }
                // }
            }
        }
    }
}
