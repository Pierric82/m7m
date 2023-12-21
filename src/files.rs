
use anyhow::{anyhow, Result};
use std::time::Duration;
use std::io::prelude::*;
use std::fs;


pub fn read_file_with_retries(path: &String, retries: Option<u8>, retry_interval: Option<Duration>) -> Result<String> {
    let contents_result = fs::read_to_string(path);
    match (contents_result, retries) {
        (Ok(s),_) => Ok(s),
        (Err(_), Some(0)) | (Err(_), None) => Err(anyhow!("could not read file {}", path)),
        (Err(_), _) => {
            let interval = retry_interval.unwrap_or(Duration::new(1,0));
            log::debug!("could not read file {}, retrying in {} seconds", path,interval.as_secs()); 
            std::thread::sleep(interval);
            read_file_with_retries(path, retries.map(|r| r - 1), retry_interval)
        }
    }
}

fn append_to_file(path: &str, input: &str) -> Result<()> {
    let file = fs::OpenOptions::new().write(true).create(true).append(true).open(path).map_err(|_| anyhow!("could not open file {} for writing",path))?;
    writeln!(&file, "{}", input).map_err(|_| anyhow!("could not write to file {}", path))
}

pub fn append_to_file_with_retries(path: &String, input: &str, retries: Option<u8>, retry_interval: Option<Duration>) -> Result<()> {
    match (append_to_file(path, input), retries) {
        (Ok(()),_) => Ok(()),
        (Err(_), Some(0)) | (Err(_), None) => Err(anyhow!("could not write to file {}", path)),
        (Err(e), _) => {
            let interval = retry_interval.unwrap_or(Duration::new(1,0));
            log::debug!("could not write to file {} due to {}, retrying in {} seconds", path,e,interval.as_secs()); 
            std::thread::sleep(interval);
            append_to_file_with_retries(path, input, retries.map(|r| r - 1), retry_interval)
        }
    }
}