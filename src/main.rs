pub mod model;
pub mod processor;
pub mod utils;

use std::{process};
use crate::processor::Processor;
use crate::utils::{get_first_arg, print_account_data};

fn main() {
    match get_first_arg() {
        Ok(file_path) => {
            match Processor::process_file(file_path) {
                Ok(processor) => {
                    if let Err(err) = print_account_data(processor) {
                        eprintln!("{}", err);
                        process::exit(1);
                    }
                }
                Err(err) => {
                    eprintln!("{}", err);
                    process::exit(1);
                }
            }
        }
        Err(err) => {
            eprintln!("{}", err);
            process::exit(1);
        }
    }
}


