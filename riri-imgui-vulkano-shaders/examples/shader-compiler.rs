use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::path::PathBuf;
use shaderc::{ShaderKind, SourceLanguage};
use riri_imgui_vulkano_shaders::AppCompiler;

#[derive(Debug)]
pub enum CompilerError {
    NotEnoughArguments,
    InputFileMissing(String),
    MissingFileName(String),
    InvalidFileNameFormat(String),
    UnknownShaderType(String),
    UnknownSourceLangauge(String),
    NoParentDirectory(String),
    UnknownFlag(String)
}

impl Error for CompilerError {}

impl Display for CompilerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            CompilerError::NotEnoughArguments => write!(f, "Not enough arguments were given.\n\
            Compiler usage:\n\
            cargo run --example shader-compiler -- [in file] [out file] (flags)"),
            CompilerError::InputFileMissing(p) => write!(f, "Input file \"{}\" does not exist.", p),
            CompilerError::InvalidFileNameFormat(p) => write!(f, "Invalid format for {}. Input shaders should be named as [name].[shader type].[language]\n\
            Valid options for shader type are: vs (vertex), ps (pixel), cs (compute), gs (geometry)\n\
            Valid options for source language are: glsl, hlsl", p),
            CompilerError::UnknownShaderType(p) => write!(f, "Unknown shader type {}\n\
            Valid options are: vs (vertex), ps (pixel), cs (compute), gs (geometry)", p),
            CompilerError::UnknownSourceLangauge(p) => write!(f, "Unknown source language {}\n\
            Valid options are: glsl, hlsl", p),
            CompilerError::UnknownFlag(p) => write!(f, "Unknown flag {}\n\
            Valid options are: werror, debug", p),
            _ => <Self as Debug>::fmt(self, f)
        }
    }
}

fn main() {
    if let Err(e) = executor(std::env::args().collect()) {
        println!("ERROR: {}", e);
    }
}

fn executor(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    if args.len() < 3 {
        return Err(Box::new(CompilerError::NotEnoughArguments));
    }
    let (fin, fout) = (PathBuf::from(&args[1]), PathBuf::from(&args[2]));
    // extra arguments
    let mut warnings_are_errors = false;
    let mut generate_debug_info = false;
    for arg in &args[3..] {
        match arg.as_ref() {
            "werror" => warnings_are_errors = true,
            "debug" => generate_debug_info = true,
            _ => ()
        }
    }
    let (str_in, str_out) = (fin.to_str().unwrap().to_string(), fout.to_str().unwrap().to_string());
    if !std::fs::exists(fin.as_path())? {
        return Err(Box::new(CompilerError::InputFileMissing(str_in)));
    }
    let filename = fin.file_name().ok_or(CompilerError::MissingFileName(str_in))?.to_str().unwrap().to_string();
    let parts: Vec<&str> = filename.split(".").collect();
    if parts.len() != 3 {
        return Err(Box::new(CompilerError::InvalidFileNameFormat(filename)));
    }
    let shader_type = match parts[1] {
        "vs" => ShaderKind::Vertex,
        "ps" => ShaderKind::Fragment,
        "cs" => ShaderKind::Compute,
        "gs" => ShaderKind::Geometry,
        v => return Err(Box::new(CompilerError::UnknownShaderType(v.to_string())))
    };
    let language = match parts[2] {
        "glsl" => SourceLanguage::GLSL,
        "hlsl" => SourceLanguage::HLSL,
        v => return Err(Box::new(CompilerError::UnknownSourceLangauge(v.to_string())))
    };
    let bytes = AppCompiler::from_path(fin.as_path())?
        .set_shader_kind(shader_type)?
        .set_source_language(language)
        .set_warnings_as_errors(warnings_are_errors)
        .set_generate_debug_info(generate_debug_info)
        .write_to_vec()?;
    std::fs::create_dir_all(fout.parent().ok_or(CompilerError::NoParentDirectory(str_out))?)?;
    std::fs::write(fout.as_path(), bytes)?;
    Ok(())
}