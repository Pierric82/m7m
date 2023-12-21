#![allow(dead_code)]

use std::env;
use std::sync::Mutex;
use anyhow::{anyhow,Result};

/// A Notifier is one channel that is set up to deliver an alert.
pub trait Notifier: Sync + Send{
  fn send_message(&self, msg: &str) -> Result<()>;
  fn chain_with(self, other: Box<dyn Notifier>) -> NotifierChain
  where Self: Sized + 'static + Sync
  {
      let nc = NotifierChain::new().chain_with(Box::new(self));
      nc.chain_with(other)
  }
}

/// The TelegramNotifier is a specific implementation of Notifier that sends its message via Telegram.
pub struct TelegramNotifier {
  chat_id: i64,
  token: String
}

impl TelegramNotifier {
  pub fn new(token: String, chat_id: i64) -> Self {
    Self { token, chat_id }
  }

  pub fn default() -> Self {
    let token = env::var("T_TOKEN").expect("need T_TOKEN variable");
    let chat_id: i64 = env::var("T_CHAT_ID").expect("need T_CHAT_ID variable").parse().expect("T_CHAT_ID not parseable to an integer");
    Self { token, chat_id }
  }

}

impl Notifier for TelegramNotifier {
  fn send_message(&self, msg: &str) -> Result<()> {
    let response = telegram_notifyrs::send_message(msg.to_owned(), &self.token, self.chat_id);
    if response.error() { Err(anyhow!("telegram notification returned an error")) }
    else {Ok(())}
  }
}

pub struct PrintNotifier {}
impl PrintNotifier {
  pub fn new() -> Self {
    Self {}
  }
}
impl Notifier for PrintNotifier {
  fn send_message(&self, msg: &str) -> Result<()> {
    println!("{}",msg);
    Ok(())
  }
}

pub struct MemoryNotifier {
 saved_messages: Mutex<Vec<String>>
}
impl Notifier for MemoryNotifier {
  fn send_message(&self, msg: &str) -> Result<()> {
    self.saved_messages.lock().unwrap().push(msg.to_owned());
    Ok(())
  }
}
impl MemoryNotifier {
  pub fn new() -> Self {
    Self { saved_messages: Mutex::new(Vec::new()) }
  }
  pub fn get_saved_messages(&self) -> Vec<String> {
    self.saved_messages.lock().unwrap().clone()
  }
}

/// A NotifierChain can be set up with any number of Notifier instances, and will send its messages to all of them.
pub struct NotifierChain {
  notifiers: Vec<Box<dyn Notifier>>
}

impl Notifier for NotifierChain {
  fn send_message(&self, msg: &str) -> Result<()> {
    for notifier in &self.notifiers {
      notifier.send_message(msg)?;
    } 
    Ok(())
  }
  fn chain_with(mut self, notifier: Box<dyn Notifier>) -> Self {
    self.notifiers.push(notifier);
    self
  }
}

impl NotifierChain {
  pub fn new() -> Self {
    Self {
      notifiers: Vec::new()
    }
  }

  /// The default chain is set up with one TelegramNotifier using defaults from the environment
  pub fn default() -> Self {
    Self::new()
    .chain_with(Box::new(TelegramNotifier::default()))
  }
}



#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_default_chain() {
    let dc = NotifierChain::default();
    assert_eq!(dc.notifiers.len(),1);
  }

  #[test]
  fn test_builder() {
    let c = 
      NotifierChain::new()
      .chain_with(Box::new(PrintNotifier::new()))
      .chain_with(Box::new(MemoryNotifier::new()));
    assert_eq!(c.notifiers.len(),2);    
  }

  #[test]
  fn test_memory_send() {
    let n = MemoryNotifier::new();
    let _ = n.send_message("1");
    let _ = n.send_message("2");
    assert_eq!(n.get_saved_messages().len(),2);
  }

  #[test]
  fn test_builder_from_notifier() {
    let c = 
      PrintNotifier::new()
      .chain_with(Box::new(MemoryNotifier::new()))
      .chain_with(Box::new(TelegramNotifier::default()));
    assert_eq!(c.notifiers.len(),3);
  }
}
