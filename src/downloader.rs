#![allow(dead_code)]
use std::path::PathBuf;

struct Downloader {
  target_dir: PathBuf,
  session: String,
  cache: Option<Cache>
}

impl Downloader {
  fn get_html_nocache(&self, _url: String, _local_name: Option<String>) -> Result<String,String> {
    todo!()
  }
  
  fn get_html(&self, url: String, local_name: Option<String>) -> Result<String,String> {
    if let Some(cache) = &self.cache {
      let cached_html = cache.get_html(url.clone(), local_name.clone()).ok();
      if let Some(cached_html) = cached_html {
        return Ok(cached_html)
      }
      else {
        let html = self.get_html_nocache(url, local_name)?;
        // TODO: add to cache
        Ok(html)
      }
    }
    else {
      self.get_html_nocache(url, local_name)
    }
  }
}

struct Cache {
  cache_dir: PathBuf,
}

impl Cache {
  fn get_html(&self, _url: String, _local_name: Option<String>) -> Result<String,String> {
    todo!()
  }

  fn get_file(&self, _url: String, _local_name: Option<String>) -> Result<String, String> {
    todo!()
  }
}