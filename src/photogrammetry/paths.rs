use std::path::PathBuf;
use std::str::FromStr;

pub fn texture_folder() -> PathBuf {parent_folder().join("odm_texturing")}

pub fn parent_folder() -> PathBuf { PathBuf::from_str("/ph").unwrap() }

pub fn archive_file() -> PathBuf { PathBuf::from_str("/model.zip").unwrap() }

pub fn image_folder() -> PathBuf { parent_folder().join("images") }
