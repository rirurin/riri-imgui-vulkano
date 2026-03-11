use std::path::{Path, PathBuf};
use riri_mod_tools::{git_version, mod_package };
use shaderc::{ShaderKind, SourceLanguage};
#[cfg(target_os = "windows")]
use winresource::WindowsResource;
use riri_imgui_vulkano_shaders::AppCompiler;

#[cfg(target_os = "windows")]
fn get_appicon<P>(base: P, icon_dim: u32) -> PathBuf
where P: AsRef<Path> {
    base.as_ref()
        .join("data")
        .join(format!("appicon_{}.ico", icon_dim))
}

fn get_project_root<P>(base: P) -> PathBuf
where P: AsRef<Path> {
    base.as_ref()
        .parent().unwrap()
        .join("Cargo.toml")
}

fn get_output_directory() -> PathBuf {
    PathBuf::from(std::env::var("OUT_DIR").unwrap())
        .parent().unwrap()
        .parent().unwrap()
        .parent().unwrap()
        .to_owned()
}

fn main() {
    let base = std::env::current_dir().unwrap();
    let out_dir = get_output_directory();
    let shader_names = [
        "shaders/basic3d.ps",
        "shaders/basic3d.vs",
        "shaders/imgui.ps",
        "shaders/imgui.vs",
        "shaders/phong.ps",
        "shaders/phong.vs",
    ];
    let shaders_in = shader_names.map(|v| {
        let (name, ext) = v.rsplit_once(".").unwrap();
        let src_ext = format!("{}.{}.glsl", name, ext);
        base.join(&src_ext)
    });
    let shaders_out = shader_names.map(|v| {
        let (base, ext) = v.rsplit_once(".").unwrap();
        let spirv_ext = format!("{}.{}.spv", base, ext);
        out_dir.join(&spirv_ext)
    });
    for shader in &shaders_in {
        println!("cargo::rerun-if-changed={}", shader.to_str().unwrap());
    }
    for (path_in, path_out) in shaders_in.iter().zip(shaders_out.iter()) {
        // from shader-compiler example in riri-imgui-vulkano-shadesr
        let filename = path_in.file_name().unwrap().to_str().unwrap().to_string();
        let parts: Vec<&str> = filename.split(".").collect();
        let shader_type = match parts[1] {
            "vs" => ShaderKind::Vertex,
            "ps" => ShaderKind::Fragment,
            "cs" => ShaderKind::Compute,
            "gs" => ShaderKind::Geometry,
            v => panic!("Unknown shader type {}", v)
        };
        let language = match parts[2] {
            "glsl" => SourceLanguage::GLSL,
            "hlsl" => SourceLanguage::HLSL,
            v => panic!("Unknown source language {}", v)
        };
        let bytes = AppCompiler::from_path(path_in.as_path()).unwrap()
            .set_shader_kind(shader_type).unwrap()
            .set_source_language(language)
            .set_warnings_as_errors(false)
            .set_generate_debug_info(false)
            .write_to_vec().unwrap();
        std::fs::create_dir_all(path_out.parent().unwrap()).unwrap();
        std::fs::write(path_out.as_path(), bytes).unwrap();
    }
    let cargo_info = mod_package::CargoInfo::new_with_resolver(base.as_path(), get_project_root).unwrap();
    #[cfg(target_os = "windows")]
    {
        let appicon = get_appicon(base.as_path(), 256);
        println!("cargo::rerun-if-changed={}", appicon.to_str().unwrap());
        let mut win_res = WindowsResource::new();
        let version = cargo_info.get_package_string_required("version").unwrap();
        win_res.set("FileVersion", version);
        win_res.set("FileDescription", "Riri Imgui Vulkano Sample App");
        let version_parts: Vec<u64> = version.split(".").enumerate()
            .filter_map(|(i, v)| if i < 3 { v.parse::<u64>().ok() } else { None })
            .collect();
        let version_id = (version_parts.get(0).unwrap_or(&0) << 0x30)
            | (version_parts.get(1).unwrap_or(&0) << 0x20)
            | (version_parts.get(2).unwrap_or(&0) << 0x10);
        win_res.set_version_info(winresource::VersionInfo::PRODUCTVERSION, version_id);
        win_res.set_language(0x409);
        win_res.set_icon_with_id(appicon.to_str().unwrap(), "101");
        win_res.compile().unwrap();
    }
    // Get version info
    git_version::create_version_file(&base, cargo_info.get_package_string_required("version").unwrap());
}