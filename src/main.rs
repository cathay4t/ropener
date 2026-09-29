// SPDX-License-Identifier: GPL-3.0-or-later

use std::{env::args, fs, fs::File, io::Read, process::Command, str};

use toml::Table;
use url::form_urlencoded;

static FILE_SPLITER: &str = ": ";
static GIO_CONTENT_TYPE: &str = "standard::content-type: ";
static CFG_GLOBAL: &str = "global";
static DEFAULT_FILE_TYPE: &str = "default";

fn get_cfg() -> Table {
    let home_path = std::env::var("HOME").expect("Failed to find HOME path");
    let mut fd = File::open(format!("{}/.config/ropener.conf", home_path))
        .expect("Failed to open config file");
    let mut contents = String::new();
    fd.read_to_string(&mut contents)
        .expect("Failed to read config file");
    contents
        .parse::<Table>()
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

fn split_file_type(file_type: &str) -> (String, String) {
    let file_types: Vec<&str> = file_type.split('/').collect();
    (
        file_types[0].trim().to_string(),
        file_types[1].trim().to_string(),
    )
}

fn get_gio_file_type(file_path: &str) -> Option<(String, String)> {
    let result = Command::new("gio")
        .arg("info")
        .arg("-a")
        .arg("standard::content-type")
        .arg(file_path)
        .output()
        .ok()?;
    if !result.status.success() {
        return None;
    }
    let output = str::from_utf8(&result.stdout).ok()?;
    let index = output.find(GIO_CONTENT_TYPE)?;
    let file_type = output[index + GIO_CONTENT_TYPE.len()..].lines().next()?;
    Some(split_file_type(file_type))
}

fn get_file_file_type(file_path: &str) -> (String, String) {
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
    split_file_type(&output[index + FILE_SPLITER.len()..])
}

fn get_file_type(file_path: &str) -> (String, String) {
    get_gio_file_type(file_path)
        .unwrap_or_else(|| get_file_file_type(file_path))
}

fn get_cmd(cfg: &Table, main_file_type: &str, sub_file_type: &str) -> String {
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
    form_urlencoded::parse(&file_uri.as_bytes()["file://".len()..])
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

    if !std::path::Path::new(&file_path).exists() {
        eprintln!("File {} does not exists", &file_path);
        std::process::exit(1);
    }

    let (main_file_type, sub_file_type) = get_file_type(&file_path);
    let cmd = get_cmd(&get_cfg(), &main_file_type, &sub_file_type);
    println!(
        "{}\n{}/{}\n{}",
        file_path, main_file_type, sub_file_type, cmd
    );
    let cmds: Vec<&str> = cmd.split(" ").collect();

    let cmd = cmds[0];
    let mut args: Vec<&str> = cmds[1..].to_vec();
    args.push(file_path.as_str());

    Command::new(cmd)
        .args(&args)
        .spawn()
        .unwrap_or_else(|_| panic!("failed to execute {}", cmd))
        .wait()
        .unwrap_or_else(|_| panic!("failed to execute {}", cmd));
    println!();
}
