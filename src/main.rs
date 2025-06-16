use rayon::iter::ParallelIterator;
pub mod model;
pub mod processor;
pub mod utils;

use std::{env, error::Error, process};
use rayon::prelude::ParallelBridge;
use crate::utils::{get_first_arg, process_file};

fn main() {
    match get_first_arg() {
        Ok(file_path) => {
            if let Err(err) = process_file(file_path) {
                println!("{}", err);
                process::exit(1);
            }
        }
        Err(err) => {
            println!("{}", err);
            process::exit(1);
        }
    }
}


