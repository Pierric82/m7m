mod notifiers;
mod triggers;
mod data;
mod yamlworkflow;
mod workflow;
mod files;
use std::env;
use anyhow::Result;

struct CLIConfig {
  paths: Vec<String>,
  flow_names: Vec<String>
}

fn show_usage_and_quit() {
  println!(r#"
Usage:
  m7m [options] flow_file1.yml flow_file2.yml ...

Whereby options can be:
  -o flow_name1,flow_name2,flow_name3...
     limit the flows to execute to the given list, even if other flows are found in the flow files

  "#);
  std::process::exit(1);
}
fn parse_cli() -> CLIConfig {
  let mut paths = vec![];
  let mut flow_names = vec![];

  let mut args = Box::new(env::args().skip(1));
  while let Some(arg) = args.next() {
    if arg == "-o" {
      if let Some(list) = args.next() {
        flow_names = list.split(',').map(|s| s.trim().to_string()).collect()
      }
      else {show_usage_and_quit()}
    }
    else {
      paths.push(arg.clone());
    }
  }

  if paths.is_empty() { show_usage_and_quit(); }

  CLIConfig {
    paths,
    flow_names
  }
}

fn select_flow(flow_name: &Option<String>, flow_names: &Vec<String>)  -> bool {
  if flow_names.is_empty() {true}
  else {
    match flow_name {
      None => false,
      Some(name) => flow_names.contains(name)
    }
  }
}

fn main() -> Result<()> {
  env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).format_target(false).init();
  let config = parse_cli();

  let mut handles = vec![];
  for path in config.paths {
    log::debug!("loading file {}", &path);
    let flows = yamlworkflow::YamlWorkflow::flows_from_file(&path)?;
    for flow in flows {
      if select_flow(&flow.flow_name, &config.flow_names) {
        handles.append(&mut workflow::start(flow));
      }
    }

  }

  if handles.is_empty() {
    log::error!("No valid flow found in arguments.");
  }
  else {
    for handle in handles {
      let _ = handle.join().expect("a thread panicked");
    }
  }

  Ok(())
}