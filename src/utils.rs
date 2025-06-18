use std::{env};
use std::error::Error;
use std::ffi::OsString;
use std::io::{stdout, Write};
use crate::processor::Processor;

pub fn get_first_arg() -> Result<OsString, Box<dyn Error>> {
    match env::args_os().nth(1) {
        None => Err(From::from("expected 1 argument, but got none")),
        Some(file_path) => Ok(file_path)
    }
}

pub fn print_account_data(processor: Processor) -> Result<(), Box<dyn Error>>{
    let mut lock = stdout().lock();
    writeln!(lock, "client, available, held, total, locked")?;
    for account_data in processor.accounts() {
        let account_key = account_data.key();
        let account_string = format!("{:?}, {:?}, {:?}, {:?}, {:?}\n",
                                     account_key,
                                     account_data.available(),
                                     account_data.held(),
                                     account_data.total(),
                                     account_data.locked()
        );
        write!(lock, "{}", account_string.as_str())?;
    }
    stdout().flush()?;
    Ok(())
}
