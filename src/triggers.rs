#![allow(dead_code)]
use std::time::Duration;
use std::thread;
use anyhow::{anyhow,Error};

pub type Thread = thread::JoinHandle<Result<(),Error>>;
pub trait Trigger {
  fn join(self) -> Result<(),Error>;
  fn thread(self: Box<Self>) ->  Thread;
}

pub struct IntervalTrigger {
  pub duration: Duration,
  pub thread: thread::JoinHandle<Result<(),Error>>
}

impl IntervalTrigger {
  pub fn seconds<F>(number: usize, action: F) -> Self
    where F: Fn() -> Result<(),Error> + Send + 'static
    {
      let duration = Duration::from_secs_f64(number as f64);
      Self::duration(duration, action)
    }
  
  pub fn minutes<F>(number: usize, action: F) -> Self
    where F: Fn() -> Result<(),Error> + Send + 'static
    {
      let duration = Duration::from_secs_f64(60f64 * number as f64);
      Self::duration(duration, action)
    }
  
  pub fn duration<F>(duration: Duration, action: F) -> Self
    where F: Fn() ->  Result<(),Error> + Send + 'static
  {
    let thread = thread::spawn(move || {
      loop {
        let _ = action(); // note that we don't forward the error as this would end the thread and thus the loop, we want to keep it going
        log::debug!("Next run will start in {} seconds", duration.as_secs());
        thread::sleep(duration);
      }
    });
    Self {duration, thread}//)
  }
}

impl Trigger for IntervalTrigger {
  fn join(self) -> Result<(),Error> {
    self.thread.join()
    .unwrap_or(Err(anyhow!("join error")))
  }
  fn thread(self: Box<Self>) -> Thread {
    self.thread
  }
}

pub struct OnceTrigger {
  pub thread: thread::JoinHandle<Result<(),Error>>
}
impl Trigger for OnceTrigger {
  fn join(self) -> Result<(),Error> {
    self.thread.join()
    .unwrap_or(Err(anyhow!("join error")))
  }
  fn thread(self: Box<Self>) -> Thread {
    self.thread
  }
}
impl OnceTrigger {
  pub fn new<F>(action: F) -> Self
    where F: Fn() ->  Result<(),Error> + Send + 'static
  {
    let thread = thread::spawn(move || {
      let _ = action();
      Ok(())
    });
    Self { thread }
  }
}