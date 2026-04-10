use std::path::{Path, PathBuf};
use riri_mod_tools::{git_version, mod_package };
#[cfg(feature = "use_compiler")]
use shaderc::{ShaderKind, SourceLanguage};
#[cfg(target_os = "windows")]
use winresource::WindowsResource;
#[cfg(feature = "use_compiler")]
use riri_imgui_vulkano_shaders::AppCompiler;

fn get_appicon<P>(base: P, icon_dim: u32) -> PathBuf
where P: AsRef<Path> {
    #[cfg(target_os = "windows")]
    let target_file = format!("appicon_win32_{}.ico", icon_dim);
    #[cfg(not(target_os = "windows"))]
    let target_file = format!("appicon_x11_{}.png", icon_dim);
    base.as_ref()
        .join("data")
        .join(target_file)
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
    // Compile shaders and copy into the crate's /shaders folder so we can push the compiled
    // bytecode to the repo. This avoids having to bundle shaderc which slows down compilation
    // by a lot.
    let shader_names = [
        "shaders/basic3d.ps",
        "shaders/basic3d.vs",
        "shaders/imgui.ps",
        "shaders/imgui.vs",
        "shaders/phong.ps",
        "shaders/phong.vs",
    ];
    #[cfg(feature = "use_compiler")]
    let shaders_in = shader_names.map(|v| {
        let (name, ext) = v.rsplit_once(".").unwrap();
        let src_ext = format!("{}.{}.glsl", name, ext);
        base.join(&src_ext)
    });
    let shaders_out = shader_names.map(|v| {
        let (name, ext) = v.rsplit_once(".").unwrap();
        let spirv_ext = format!("{}.{}.spv", name, ext);
        base.join(&spirv_ext)
    });
    let shaders_target = shader_names.map(|v| {
        let (name, ext) = v.rsplit_once(".").unwrap();
        let spirv_ext = format!("{}.{}.spv", name, ext);
        out_dir.join(&spirv_ext)
    });
    std::fs::create_dir_all(out_dir.join("shaders")).unwrap();
    #[cfg(feature = "use_compiler")]
    for shader in &shaders_in {
        println!("cargo::rerun-if-changed={}", shader.to_str().unwrap());
    }

    #[cfg(feature = "use_compiler")]
    for (path_in, path_out) in shaders_in.iter().zip(shaders_out.iter()) {
        // from shader-compiler example in riri-imgui-vulkano-shaders
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
    // Copy compiled shaders to output directory
    for (path_in, path_out) in shaders_out.iter().zip(shaders_target.iter()) {
        std::fs::copy(path_in, path_out).unwrap();
    }
    // Copy fonts to output directory
    let font_names = [
        "data/LibreBodoni-Bold.ttf",
        "data/NotoSansCJKjp-Medium.otf"
    ];
    let fonts_in = font_names.map(|v| base.join(v));
    let fonts_out = font_names.map(|v| out_dir.join(v));
    std::fs::create_dir_all(out_dir.join("data")).unwrap();
    for (font_in, font_out) in fonts_in.iter().zip(fonts_out.iter()) {
        std::fs::copy(font_in, font_out).unwrap();
    }
    // Setup Windows app info
    let cargo_info = mod_package::CargoInfo::new_with_resolver(base.as_path(), get_project_root).unwrap();
    let appicon = get_appicon(base.as_path(), 256);
    println!("cargo::rerun-if-changed={}", appicon.to_str().unwrap());
    #[cfg(target_os = "windows")]
    {
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
    // Setup Linux app info
    #[cfg(target_os = "linux")]
    {
        std::fs::copy(appicon, out_dir.join("data/appicon.png")).unwrap();
    }
    // Get version info
    git_version::create_version_file(&base, cargo_info.get_package_string_required("version").unwrap());
}