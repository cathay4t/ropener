//
// Copyright (C) 2019 Gris Ge
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.
//
// Author: Gris Ge <cnfourt@gmail.com>

use std::env::args;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::process::Command;
use std::str;
use toml::Value;
use url::form_urlencoded;

static FILE_SPLITER: &str = ": ";
static CFG_GLOBAL: &str = "global";
static DEFAULT_FILE_TYPE: &str = "default";

fn get_cfg() -> Value {
    let mut fd = File::open("/home/fge/.ropener.conf")
        .expect("Failed to open config file");
    let mut contents = String::new();
    fd.read_to_string(&mut contents)
        .expect("Failed to read config file");
    contents
        .parse::<Value>()
        .expect("Failed to parse config file")
}

fn is_soft_link(file_path: &str) -> bool {
    if let Ok(metadata) = fs::symlink_metadata(file_path) {
        metadata.file_type().is_symlink()
    } else {
        false
    }
}

fn get_soft_link_source(file_path: &str) -> String {
    if let Ok(source) = fs::read_link(file_path) {
        if source.is_absolute() {
            source.to_str().unwrap().into()
        } else {
            let mut file_dir = std::path::PathBuf::from(file_path);
            file_dir.pop();
            let ab_source = std::path::PathBuf::from(format!(
                "{}/{}",
                file_dir.to_str().unwrap(),
                source.to_str().unwrap(),
            ));
            ab_source.to_str().unwrap().into()
        }
    } else {
        file_path.into()
    }
}

fn get_file_type(file_path: &str) -> (String, String) {
    let result = Command::new("file")
        .arg("-E")
        .arg("--mime-type")
        .arg(file_path)
        .output()
        .expect("failed to execute file command");
    if !result.status.success() {
        panic!(
            "Failed to read mime type of {}: error {}: {}",
            file_path,
            result.status.code().unwrap(),
            str::from_utf8(&result.stdout).unwrap()
        )
    }
    let output: String = String::from_utf8(result.stdout)
        .expect("Failed to convert file command output to String");
    let index = output
        .find(FILE_SPLITER)
        .expect("Failed to find ': ' in file output");
    let file_type = &output[index + FILE_SPLITER.len()..];
    let file_types: Vec<&str> = file_type.split('/').collect();
    (
        file_types[0].trim().to_string(),
        file_types[1].trim().to_string(),
    )
}

fn get_cmd(cfg: &Value, main_file_type: &str, sub_file_type: &str) -> String {
    let cfg = cfg.as_table().unwrap();
    let global_cfg = match cfg.get(CFG_GLOBAL) {
        Some(c) => c.as_table().unwrap(),
        None => panic!("No global config"),
    };
    let global_default_cmd = match global_cfg.get(DEFAULT_FILE_TYPE) {
        Some(c) => c.as_str().unwrap().to_string(),
        None => panic!("No default config in global"),
    };

    let cmd = match cfg.get(main_file_type) {
        Some(sub_cfg) => match sub_cfg.get(sub_file_type) {
            Some(c) => c.as_str().unwrap().to_string(),
            None => match sub_cfg.get(DEFAULT_FILE_TYPE) {
                Some(c) => c.as_str().unwrap().to_string(),
                None => global_default_cmd,
            },
        },
        None => global_default_cmd,
    };
    match global_cfg.get(&cmd) {
        Some(c) => c.as_str().unwrap().to_string(),
        None => cmd,
    }
}

fn decode_file_uri(file_uri: &str) -> String {
    form_urlencoded::parse(file_uri["file://".len()..].as_bytes())
        .map(|(key, val)| [key, val].concat())
        .collect()
}

fn main() {
    let argv: Vec<String> = args().collect();

    if argv.len() < 2 {
        panic!("Need file path");
    }

    let mut file_path = if argv[1].starts_with("file://") {
        decode_file_uri(&argv[1])
    } else {
        argv[1].clone()
    };

    if is_soft_link(&file_path) {
        file_path = get_soft_link_source(&file_path);
    }

    if ! std::path::Path::new(&file_path).exists() {
        eprintln!("File {} does not exists", &file_path);
        std::process::exit(1);
    }


    let (main_file_type, sub_file_type) = get_file_type(&file_path);
    let cmd = get_cmd(&get_cfg(), &main_file_type, &sub_file_type);
    println!(
        "{}\n{}/{}\n{}",
        file_path, main_file_type, sub_file_type, cmd
    );

    Command::new("bash")
        .arg("-c")
        .arg(&format!("{} '{}'", cmd, file_path))
        .spawn()
        .unwrap_or_else(|_| panic!("failed to execute {}", cmd))
        .wait()
        .unwrap_or_else(|_| panic!("failed to execute {}", cmd));
}
