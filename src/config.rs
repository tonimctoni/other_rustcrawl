use serde_json;

use std::sync;
use std::fs;


#[derive(Deserialize, Clone)]
pub struct FileConfig {
    pub mimetype: String,
    pub file_prefix: String,
    pub file_extension: String,
    pub folder_name: String,
}

#[derive(Deserialize)]
pub struct Config {
    pub initial_urls: Vec<String>,
    pub report_period: u64,
    pub client_threads: usize,

    pub timeout: u64,
    pub max_body_len: usize,
    pub buffer_size: usize,

    pub max_urls_per_html: usize,
    pub max_url_hosts_per_html: usize,

    pub files_to_gather: Vec<FileConfig>,
}


impl Config {
    pub fn get_or_panic(json_config_filename: &str) -> sync::Arc<Config>{
        let json_file=fs::File::open(json_config_filename).unwrap();
        let config: Config=serde_json::from_reader(json_file).unwrap();

        assert!(config.initial_urls.len()>0);
        assert!(config.initial_urls.iter().all(|url| url.len()>0));
        assert!(config.client_threads>0);
        assert!(config.timeout>0);
        assert!(config.max_body_len>0);
        assert!(config.buffer_size>0);
        assert!(config.max_urls_per_html>0);
        assert!(config.max_url_hosts_per_html>0);
        assert!(config.files_to_gather.iter().all(|file_config| file_config.mimetype.len()>0));
        assert!(config.files_to_gather.iter().all(|file_config| file_config.file_extension.len()>0));
        assert!(config.files_to_gather.iter().all(|file_config| file_config.folder_name.len()>0));

        sync::Arc::new(config)
    }
}