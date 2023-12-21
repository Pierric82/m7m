#![allow(dead_code)]
use std::{io::BufReader, fs::File, collections::HashMap, time::Duration};

use serde::Deserialize;
use anyhow::{anyhow, Result};


#[derive(Debug, Deserialize )]
pub struct Trigger {
    #[serde(rename = "type")]
    pub trigger_type: String,
    #[serde(deserialize_with = "optional_duration_parser")]
    #[serde(default)]
    pub interval: Option<Duration>,
}

#[derive(Debug, Deserialize )]
pub struct Notifier {
    pub name: String,
    #[serde(rename = "type")]
    pub notifier_type: String,
    pub token: Option<String>,
    pub chat_id: Option<String>,
}

#[derive(Debug, Deserialize, Clone )]
#[serde(rename_all ="snake_case")]
pub enum Step {
    GetUrl {
        url: String,
        output_var: Option<String>,
        #[serde(flatten)]
        fail_spec: FailSpec
    },
    PostUrl {
        url: String,
        body: String,
        #[serde(default)]
        headers: HashMap<String,String>,
        #[serde(flatten)]
        fail_spec: FailSpec,
    },
    TextExtractOneCapture {
        input_var: Option<String>,
        output_var: Option<String>,
        regex: String,
        #[serde(flatten)]
        fail_spec: FailSpec,
    },
    CompareVar {
        input_var: Option<String>,
        compare_with: String,
        compare_for: String,
        #[serde(default)]
        if_true: Vec<Step>,
        #[serde(default)]
        if_false: Vec<Step>,
    },
    Notify {
        notifier: String,
        message: String,
        #[serde(flatten)]
        fail_spec: FailSpec,
    },
    AbortFlow,
    DebugState,
    Sleep {
        #[serde(deserialize_with = "duration_parser")]
        duration: Duration
    },
    ReadFromFile {
        path: String,
        output_var: Option<String>,
        #[serde(flatten)]
        fail_spec: FailSpec,
    },
    AppendToFile {
        path: String,
        input_var: Option<String>,
        #[serde(flatten)]
        fail_spec: FailSpec,
    },
    SetVariable {
        output_var: Option<String>,
        input: String
    }
}

#[derive(Debug, Deserialize, Clone )]
pub struct FailSpec {
    pub retries: Option<u8>,
    #[serde(deserialize_with = "optional_duration_parser")]
    #[serde(default)]
    pub retry_interval: Option<Duration>,
    #[serde(default)]
    #[serde(with = "serde_yaml::with::singleton_map_recursive")]
    pub upon_failure: Vec<Step>,
}

#[derive(Debug, Deserialize )]
pub struct YamlWorkflow {
   #[serde(rename = "name")]
   pub flow_name: Option<String> ,
   pub trigger: Option<Trigger>,
    #[serde(default)]
    pub notifiers: Vec<Notifier>,
    #[serde(with = "serde_yaml::with::singleton_map_recursive")]
    pub steps: Vec<Step>,
    #[serde(default)]
    #[serde(with = "serde_yaml::with::singleton_map_recursive")]
    pub upon_failure: Vec<Step>,
}

impl YamlWorkflow {
    pub fn flows_from_file(file_path: &str) -> Result<Vec<Self>> {
        let file = File::open(file_path).unwrap();
        let buf = BufReader::new(file);
        let mut flows = vec![];
        for document in serde_yaml::Deserializer::from_reader::<BufReader<File>>(buf) {
            flows.push(YamlWorkflow::deserialize(document).map_err(|e| anyhow!("could not parse flow correctly: {}",e))?);
        }
        Ok(flows)
    }
}

fn optional_duration_parser<'de, D>(deserializer: D) -> Result<Option<std::time::Duration>, D::Error> 
where D: serde::Deserializer<'de> {
    let buf = String::deserialize(deserializer)?;
    Ok(Some(parse_duration::parse(&buf).map_err(serde::de::Error::custom)?))
}

fn duration_parser<'de, D>(deserializer: D) -> Result<std::time::Duration, D::Error> 
where D: serde::Deserializer<'de> {
    let buf = String::deserialize(deserializer)?;
    parse_duration::parse(&buf).map_err(serde::de::Error::custom)
}


#[cfg(test)]
#[test]
fn test_ext_tag() {
    let yaml = r#"
- sleep: 
    duration: 1s
- notify:
    message: hi
    notifier: test
"#;
    dbg!(serde_yaml::from_str::<Vec<Step>>(yaml).unwrap());
}


#[cfg(test)]
#[test]
fn test_duration_parsing() {
    #[derive(Debug, Deserialize )]
    struct Tester {
        #[serde(deserialize_with = "optional_duration_parser")]
        #[serde(default)]
        duration: Option<std::time::Duration>,
    }

    static EXAMPLE: &str ="
    duration: 3s
    ";

    let tester1 = serde_yaml::from_str::<Tester>(EXAMPLE).unwrap();
    if let Some(d) = tester1.duration { assert_eq!(d,Duration::new(3,0)); } else {panic!("did not parse duration correctly");}
    assert_eq!(serde_yaml::from_str::<Tester>("").unwrap().duration, None);
}
