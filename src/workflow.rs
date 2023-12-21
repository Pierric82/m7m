use regex::Regex;

use crate::{triggers::{self, Trigger}, notifiers, data, files };
use super::yamlworkflow as yw; 

use std::{collections::HashMap, sync::Arc, thread, time::Duration };
use anyhow::{anyhow,Error, bail };

#[derive(Debug,Default)]
/// A state structure for the workflow to store variables
struct State {
    last_output: Option<String>,
    vars: HashMap<String, String>,
}

impl State {
    /// Saves something as the last output and optionally saves it as a variable too
    fn set_output(&mut self, s: String, output_var: &Option<String>) {
        if let Some(var_name) = output_var {
            self.vars.insert(var_name.clone(), s.clone());
        }
        self.last_output = Some(s);
    }
    /// Retrieves a specific variable or the last output if no variable is specified
    fn get_input(&self, input_var: &Option<String>) -> Result<&String,Error> {
        if let Some(var_name) = input_var {
            self.vars.get(var_name)
            .ok_or(anyhow!("invalid input variable name for text extraction"))
        }
        else {self.last_output.as_ref().ok_or(anyhow!("no input available for text extraction")) }
    }
}

fn handle_failure_with_err(
    err: Error,
    notifiers: & HashMap<String, Box<dyn notifiers::Notifier>>,
    steps: & Vec<yw::Step>,
    state: & mut State,
    flow_name: &str
) -> Result<(), Error> {
    log::warn!("{}",err.to_string());
    if !steps.is_empty() {
        log::debug!("[{}] entering a failure sub-flow", flow_name);
        run_steps(notifiers, steps, state, flow_name)?; // if this fails too, we give up and shoot up the error
        log::debug!("[{}] Failure sub-flow completed, resuming main flow", flow_name);
        return Ok(());
    }
    Err(err) // no error handling in workflow, abort flow with error
}

fn run_steps(
    notifiers: &HashMap<String, Box<dyn notifiers::Notifier>>,
    steps: &Vec<yw::Step>,
    state: &mut State,
    flow_name: &str
) -> Result<(),Error> {
    for step in steps.iter() {
        match &step {
            
            yw::Step::AbortFlow => { log::info!("[{}] aborting flow", flow_name); bail!("Flow aborted")},
            
            yw::Step::DebugState => { log::info!("[{}] State dump: {:?}",flow_name,state); },

            yw::Step::Sleep { duration } => {
                log::info!("[{}] sleeping for {} seconds",flow_name,&duration.as_secs());
                thread::sleep(*duration);
            },

            yw::Step::Notify {notifier, message, fail_spec} => {
                log::debug!("[{}] sending notification to {}: {}",flow_name, notifier,message);
                let notifier = notifiers.get(notifier)
                    .ok_or(anyhow!("no notifier found with specified name {}", notifier))?;
                if let Err(err) = notifier.send_message(message) {
                    handle_failure_with_err(err, notifiers, &fail_spec.upon_failure, state, flow_name)?;
                }
            },

            yw::Step::GetUrl { url, output_var, fail_spec } => {
                log::debug!("[{}] Getting URL {}",flow_name,url);
                let result = data::simple_get_body(
                    url,
                    fail_spec.retries.unwrap_or(0), 
                    fail_spec.retry_interval.unwrap_or(Duration::new(1,0)).as_secs() as f64
                    ) ;

                match result {
                    Ok(s) =>  state.set_output(s, output_var),
                    Err(err) => handle_failure_with_err(err, notifiers, &fail_spec.upon_failure, state, flow_name)?
                };
            },

            yw::Step::PostUrl { url, body, headers, fail_spec } => {
                log::debug!("[{}] posting to URL {}",flow_name,url);
                let result = data::post_text(url, headers, body, 
                    fail_spec.retries.unwrap_or(0),
                    fail_spec.retry_interval.unwrap_or(Duration::new(1,0)).as_secs() as f64
                    ) ;

                if let Err(err) = result {
                    handle_failure_with_err(err, notifiers, &fail_spec.upon_failure, state, flow_name)?;
                }
            },

            yw::Step::TextExtractOneCapture { input_var, output_var, regex, fail_spec } => {
                log::debug!("[{}] applying regex: {}",flow_name,regex);
                let input = state.get_input(input_var)?;

                let re = Regex::new(regex).map_err(|_| anyhow!("error creating regex from input: {}", regex))?;

                let outcome = re.captures(input);
                match outcome {
                    Some(captures) => {
                        let mat = captures.get(1);
                        if let Some(mat) = mat {
                            state.set_output(mat.as_str().to_string(), output_var);
                        } else {
                            handle_failure_with_err(anyhow!("text not found in text extractor"), notifiers, &fail_spec.upon_failure, state, flow_name)?
                        }
                    }
                    None => handle_failure_with_err(anyhow!("text not found in text extractor"), notifiers, &fail_spec.upon_failure, state, flow_name)?
                };

            },
            
            yw::Step::CompareVar { input_var, compare_with, compare_for, if_true, if_false } => {
                log::debug!("[{}] comparing one variable: {}",flow_name, input_var.as_deref().unwrap_or("<last output>"));
                let input = state.get_input(input_var)?;

                let outcome = match compare_for.as_str() {
                    "equality" => input.as_str() == compare_with.as_str(),
                    _ => bail!("unsupported comparison type: {}",compare_for)
                };

                match (outcome, if_true.is_empty(), if_false.is_empty()) {
                    (true, false, _) => {
                        log::debug!("[{}] entering a sub-flow after comparison was true", flow_name);
                        run_steps(notifiers, if_true, state, flow_name)?;
                        log::debug!("[{}] Sub-flow completed, resuming main flow", flow_name);
                    },
                    (false, _, false) => {
                        log::debug!("[{}] entering a sub-flow after comparison was false", flow_name);
                        run_steps(notifiers, if_false, state, flow_name)?;
                        log::debug!("[{}] Sub-flow completed, resuming main flow", flow_name);
                    },
                    _ => log::debug!("[{}] no action taken as result of comparison", flow_name)
                };
                
            },

            yw::Step::ReadFromFile { path, output_var, fail_spec} => {
                log::debug!("[{}] reading from file {}", flow_name, path);
                let contents = files::read_file_with_retries(path, fail_spec.retries, fail_spec.retry_interval);
                match contents {
                    Ok(s) => state.set_output(s, output_var),
                    Err(e) => handle_failure_with_err(e, notifiers, &fail_spec.upon_failure, state, flow_name)?
                }
            },

            yw::Step::AppendToFile { path, input_var, fail_spec } => {
                log::debug!("[{}] writing (appending) to file {}", flow_name, path);
                let input = state.get_input(input_var)?;
                if let Err(e) = files::append_to_file_with_retries(path, input, fail_spec.retries, fail_spec.retry_interval) {
                    handle_failure_with_err(e, notifiers, steps, state, flow_name)?
                }
            },

            yw::Step::SetVariable { output_var, input } => {
                log::debug!("[{}] setting variable {}", flow_name, output_var.as_deref().unwrap_or("<unnamed>"));
                state.set_output(input.clone(), output_var);
            }

        }
    }

    Ok(())
}

pub fn start(yaml_workflow: yw::YamlWorkflow) -> Vec<triggers::Thread> {
    let yaml_workflow = Arc::new(yaml_workflow);
    let flow_name = yaml_workflow.flow_name.as_deref() .unwrap_or("<unnamed>") .to_string();

    let notifiers = yaml_workflow.notifiers.iter()
        .map(|n| (n.name.clone(), match n.notifier_type.as_ref() {
            "print" => Box::new(notifiers::PrintNotifier::new()) as Box<dyn notifiers::Notifier>,
            "telegram" => Box::new(notifiers::TelegramNotifier::new(
                n.token.clone().unwrap_or_else(|| panic!("[{}] telegram notifier without token", flow_name)), 
                n.chat_id.clone().unwrap_or_else(|| panic!("[{}] telegram notifier without chat id", flow_name)).parse().unwrap_or_else(|_| panic!("[{}] invalid chat id", flow_name))
            )) as Box<dyn notifiers::Notifier>,
            _ => panic!("[{}] invalid notifier type found", flow_name)
        }))
        .collect();

    let workflow_for_closure = yaml_workflow.clone();
    
    let rule  =   move || { 
        let flow_name = workflow_for_closure.flow_name.as_deref() .unwrap_or("<unnamed>") .to_string();
        log::info!("[{}] starting flow", &flow_name);

        let mut state = State::default();
        let mut outcome = run_steps(&notifiers, &workflow_for_closure.steps, &mut state, &flow_name);
        match &outcome {
            Ok(()) => log::info!("[{}] Flow completed", &flow_name),
            Err(e) => {
                if !workflow_for_closure.upon_failure.is_empty() {
                    log::warn!("[{}] Flow failed, starting fallback steps. Last error: {}", &flow_name, e);
                    outcome = run_steps(&notifiers, &workflow_for_closure.upon_failure, &mut state, &flow_name);
                    match &outcome {
                        Ok(()) => log::info!("[{}] Fallback flow completed", &flow_name),
                        Err(e) => {
                            log::warn!("[{}] Flow aborted due to error. Last error: {}", &flow_name, e)
                        }
                    }
                } else {
                    log::warn!("[{}] Flow aborted due to error. Last error: {}", &flow_name, e)
                }
            }
        };
        outcome

    };

    let triggers: Vec<Box<dyn Trigger>> = match &yaml_workflow.trigger {
        None => vec![],
        Some( yw::Trigger {trigger_type, interval}) => {
            match trigger_type.as_str() {
                "timer" => {
                    let t = triggers::IntervalTrigger::duration(interval.unwrap_or(Duration::new(1,0)),rule);
                    vec![Box::new(t)]
                },
                "once" => {
                    vec![Box::new(triggers::OnceTrigger::new(rule))]
                },
                _ => {
                    panic!("[{}] no valid trigger type found", yaml_workflow.flow_name.as_deref().unwrap_or("<unnamed>"))
                }
            }
        },
    };

    triggers.into_iter().map(|t| t.thread()).collect()
}