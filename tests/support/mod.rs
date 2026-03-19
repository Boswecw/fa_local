#![allow(dead_code)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use fa_local::SchemaName;
use serde_json::Value;

pub fn fixture_dir(kind: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("contracts")
        .join("fixtures")
        .join(kind)
}

pub fn fixture_path(kind: &str, file_name: &str) -> PathBuf {
    fixture_dir(kind).join(file_name)
}

pub fn discover_fixture_paths(kind: &str) -> Vec<PathBuf> {
    let mut paths = std::fs::read_dir(fixture_dir(kind))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"))
        .collect::<Vec<_>>();
    paths.sort();
    paths
}

pub fn load_fixture_json(kind: &str, file_name: &str) -> Value {
    let path = fixture_path(kind, file_name);
    let raw = std::fs::read_to_string(path).unwrap();
    serde_json::from_str(&raw).unwrap()
}

pub fn schema_for_fixture(path: &Path) -> SchemaName {
    let file_name = path.file_name().unwrap().to_string_lossy();
    SchemaName::all()
        .iter()
        .copied()
        .max_by_key(|schema| {
            if file_name.starts_with(schema.fixture_prefix()) {
                schema.fixture_prefix().len()
            } else {
                0
            }
        })
        .filter(|schema| file_name.starts_with(schema.fixture_prefix()))
        .unwrap_or_else(|| panic!("no schema matches fixture {}", path.display()))
}

pub fn coverage_map() -> BTreeMap<&'static str, usize> {
    SchemaName::all()
        .iter()
        .map(|schema| (schema.fixture_prefix(), 0))
        .collect()
}
