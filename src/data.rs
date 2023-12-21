use std::collections::HashMap ;

use serde_json::Value;
use reqwest::{self, StatusCode};
use anyhow::{Result, anyhow, bail};

fn handle_retry<T,F>(retries: u8, retry_secs: f64, closure: F ) 
-> Result<T>
where F: Fn() -> Result<T> {
  match (closure(), retries) {
    (Ok(r),_) => Ok(r),
    (Err(s),0) => Err(s),
    (Err(_),_) => {
      log::debug!("Error happened but {} {} left, will retry in {} seconds",
        retries,
        if retries == 1 {"retry"} else { "retries" },
        retry_secs);
      std::thread::sleep(std::time::Duration::from_secs_f64(retry_secs));
      handle_retry(retries - 1, retry_secs, closure)
    } 
  }
}

fn simple_get_response(url: &str, retries: u8, retry_secs: f64)
  -> Result<reqwest::blocking::Response> {
    
    handle_retry(retries, retry_secs, || {
      reqwest::blocking::get(url).map_err(|_| anyhow!("couldn't get URL via reqwest: {}",url))
    })
}

#[allow(dead_code)]
pub fn simple_get_json(url: &str, retries: u8, retry_secs: f64) -> Result<Value> {
  let body = simple_get_body(url, retries, retry_secs)?;
  serde_json::from_str(&body).map_err(|_| anyhow!("couldn't parse response body of {}. Body was: {}",url,&body))
}

pub fn simple_get_body(url: &str, retries: u8, retry_secs: f64) -> Result<String> {
  let res = simple_get_response(url, retries, retry_secs)?;
  if !res.status().is_success() {
    bail!("error code on GET to URL {}: {}", url, res.status());
  }
  res.text().map_err(|_| anyhow!("reqwest body error getting {}", url))
}

pub fn post_text(url: &str, headers: &HashMap<String,String>, body: &str, retries: u8, retry_secs: f64) -> Result<()> {
  handle_retry(retries, retry_secs, || {
    let mut post = reqwest::blocking::Client::new().post(url);
    for (k,v) in headers {
      post = post.header(k,v);
    }
    post = post.body(body.to_string());
    let response = post.send().map_err(|_| anyhow!("could not send POST request to {}", url))?;
    if [StatusCode::OK, StatusCode::CREATED].contains(&response.status())
      { Ok(()) } 
    else 
      { bail!("POST request to {} returned error code {}", url, response.status()) }
  })
}

#[cfg(test)]
mod tests {

use super::*;

  #[test]
  fn test_get_invalid_url() {
    let r = simple_get_body("http://www.123.45/a",1,0.5);
    let error = r.expect_err("get returned ok instead of Err with invalid url");
    log::debug!("{:?}",error);
    assert!(error.to_string().contains("couldn't get URL"));
  }

  #[test]
  fn test_get_valid_url() {
    let r = simple_get_body("https://jsonplaceholder.typicode.com/todos/1",1,0.5);
    assert!(r.is_ok());
  }

  #[test]
  /// runs the typical jsonplaceholder test
  fn test_post_text() {
    let mut hm = HashMap::new();
    hm.insert("Content-type".to_string(), "application/json; charset=UTF-8".to_string());
    let body = r#"{
      "title": "foo",
      "body": "bar",
      "userId": 1
    }"#;
    let url = "https://jsonplaceholder.typicode.com/posts";
    assert!(post_text(url,&hm,body,0,0.0).is_ok());
  }
}
