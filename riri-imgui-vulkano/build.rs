use std::path::{Path, PathBuf};
use riri_mod_tools::{git_version, mod_package};

fn get_project_root<P>(base: P) -> PathBuf
where P: AsRef<Path> {
    base.as_ref()
        .parent().unwrap()
        .join("Cargo.toml")
}

fn main() {
    let base = std::env::current_dir().unwrap();
    // Get version info
    let cargo_info = mod_package::CargoInfo::new_with_resolver(base.as_path(), get_project_root).unwrap();
    git_version::create_version_file(&base, cargo_info.get_package_string_required("version").unwrap());
}