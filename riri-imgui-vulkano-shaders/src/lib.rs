use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::path::Path;
use bitflags::bitflags;
use shaderc::{CompilationArtifact, CompileOptions, Compiler, EnvVersion, OptimizationLevel, ShaderKind, SourceLanguage, TargetEnv};

#[derive(Debug)]
pub enum CompilerError {
    NoFileName,
    UnsupportedShaderKind
}

impl Display for CompilerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        <Self as Debug>::fmt(self, f)
    }
}

impl Error for CompilerError {}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd)]
    pub struct CompilerFlags : u32 {
        const WARNINGS_AS_ERRORS = 1 << 0;
        const OPTIMIZATION_NONE = 1 << 1;
        const OPTIMIZATION_SIZE = 1 << 2;
        const OPTIMIZATION_PERF = 1 << 3;
        const SHADER_VERTEX = 1 << 4;
        const SHADER_PIXEL = 1 << 5;
        const SHADER_COMPUTE = 1 << 6;
        const SHADER_GEOMETRY = 1 << 7;
        const LANGUAGE_GLSL = 1 << 8;
        const LANGUAGE_HLSL = 1 << 9;
        const GENERATE_DEBUG_INFO = 1 << 10;
    }
}

const OPTIMIZATION_MASK: u32 = 0xe;
const SHADER_MASK: u32 = 0xf0;
const LANGUAGE_MASK: u32 = 0x300;

impl CompilerFlags {
    // WARNINGS_AS_ERRORS
    fn get_warnings_as_errors(&self) -> bool {
        self.contains(CompilerFlags::WARNINGS_AS_ERRORS)
    }

    fn set_warnings_as_errors(&mut self, value: bool) {
        self.set(CompilerFlags::WARNINGS_AS_ERRORS, value)
    }

    // OPTIMIZATION_XXXX
    fn get_optimization(&self) -> Option<OptimizationLevel> {
        let value = self.bits() & OPTIMIZATION_MASK;
        match value {
            0 => None,
            v => Some(unsafe { std::mem::transmute(v.trailing_zeros() - 1) })
        }
    }

    fn clear_optimization(&mut self) {
        *self &= !CompilerFlags::from_bits_retain(OPTIMIZATION_MASK);
    }

    fn set_optimization(&mut self, value: OptimizationLevel) {
        self.clear_optimization();
        *self |= CompilerFlags::from_bits_retain(1 << (value as u32 + 1));
    }

    // SHADER_XXXX
    fn get_shader_kind(&self) -> ShaderKind {
        unsafe { std::mem::transmute((self.bits() & SHADER_MASK).trailing_zeros() - 4) }
    }

    fn set_shader_kind(&mut self, kind: ShaderKind) -> Result<(), Box<dyn Error>> {
        *self &= !CompilerFlags::from_bits_retain(SHADER_MASK);
        *self |= match kind {
            ShaderKind::Vertex => Ok(CompilerFlags::SHADER_VERTEX),
            ShaderKind::Fragment => Ok(CompilerFlags::SHADER_PIXEL),
            ShaderKind::Compute => Ok(CompilerFlags::SHADER_COMPUTE),
            ShaderKind::Geometry => Ok(CompilerFlags::SHADER_GEOMETRY),
            _ => Err(Box::new(CompilerError::UnsupportedShaderKind))
        }?;
        Ok(())
    }

    // LANGUAGE_XXXX
    fn get_language(&self) -> SourceLanguage {
        unsafe { std::mem::transmute((self.bits() & LANGUAGE_MASK).trailing_zeros() - 8) }
    }

    fn set_language(&mut self, lang: SourceLanguage) {
        *self &= !CompilerFlags::from_bits_retain(LANGUAGE_MASK);
        *self |= CompilerFlags::from_bits_retain(1 << (lang as u32 + 8));
    }

    // DEBUG_INFO
    fn get_generate_debug_info(&self) -> bool {
        self.contains(CompilerFlags::GENERATE_DEBUG_INFO)
    }

    fn set_generate_debug_info(&mut self, value: bool) {
        self.set(CompilerFlags::GENERATE_DEBUG_INFO, value)
    }
}

#[derive(Debug)]
pub struct AppCompiler<'a> {
    flags: CompilerFlags,
    compiler: Compiler,
    source: String,
    filename: Option<String>,
    entry_point: String,
    definitions: Vec<(&'a str, &'a str)>
}

impl<'a> AppCompiler<'a> {
    /// Creates an AppCompiler instance from the given source code as a string. In this case, the
    /// filename should be manually set.
    pub fn from_string<T: AsRef<str>>(source: T) -> Result<Self, Box<dyn Error>> {
        Self::new(source.as_ref().to_string(), None)
    }

    /// Creates an AppCompiler instance from the given filename.
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn Error>> {
        let source = std::fs::read_to_string(path.as_ref())?;
        let filename = path.as_ref()
            .file_name().and_then(|v| v.to_str())
            .ok_or(CompilerError::NoFileName)?;
        Self::new(source, Some(filename.to_string()))
    }

    fn new(source: String, filename: Option<String>) -> Result<Self, Box<dyn Error>> {
        let compiler = Compiler::new()?;
        let flags = CompilerFlags::LANGUAGE_GLSL | CompilerFlags::SHADER_VERTEX;
        Ok(Self {
            flags, compiler, source, filename,
            entry_point: "main".to_string(), definitions: vec![]
        })
    }

    /// Get the filename for the shader. When using from_path, this is always set to the file path.
    /// A filename has to be manually added when using from_string
    pub fn get_filename(&self) -> Option<&str> {
        self.filename.as_ref().map(|v| v.as_str())
    }

    /// Get the name of the shader's entry point. By default, this is main
    pub fn get_entry_point(&self) -> &str {
        &self.entry_point
    }

    /// Set the filename for the shader
    pub fn set_filename<T: AsRef<str>>(mut self, name: T) -> Self {
        self.filename = Some(name.as_ref().to_string());
        self
    }

    /// Set the entry point for the shader (e.g main)
    pub fn set_entry_point<T: AsRef<str>>(mut self, name: T) -> Self {
        self.entry_point = name.as_ref().to_string();
        self
    }

    /// Sets the kind of shader that the source code describes. This is Vertex by default.
    pub fn set_shader_kind(mut self, kind: ShaderKind) -> Result<Self, Box<dyn Error>> {
        self.flags.set_shader_kind(kind)?;
        Ok(self)
    }

    /// Sets the language of the target source code file. This is usually automatically derived
    /// from the shader's file extension (.glsl or .hlsl). This is GLSL by default.
    pub fn set_source_language(mut self, value: SourceLanguage) -> Self {
        self.flags.set_language(value);
        self
    }

    /// Adds a macro definition for the shader compiler's preprocessor.
    pub fn add_macro_definition(mut self, name: &'a str, value: &'a str) -> Self {
        self.definitions.push((name, value));
        self
    }

    /// Sets the optimization level that the shader compiler should target. No setting is applied
    /// by default which lets the compiler do whatever.
    pub fn set_optimization(mut self, level: OptimizationLevel) -> Self {
        self.flags.set_optimization(level);
        self
    }

    /// Sets if the shader compiler should error out if it encounters a warning. This is off by default.
    pub fn set_warnings_as_errors(mut self, value: bool) -> Self {
        self.flags.set_warnings_as_errors(value);
        self
    }

    /// Sets if the shader compiler should generate debug info for the shader.
    pub fn set_generate_debug_info(mut self, value: bool) -> Self {
        self.flags.set_generate_debug_info(value);
        self
    }

    /// Creates a CompilationArtifact from the shader's source code and settings.
    pub fn create_artifact(&self) -> Result<CompilationArtifact, Box<dyn Error>> {
        let mut options = CompileOptions::new()?;
        options.set_target_env(TargetEnv::Vulkan, EnvVersion::Vulkan1_4 as u32);
        options.set_source_language(self.flags.get_language());
        if let Some(opt) = self.flags.get_optimization() {
            options.set_optimization_level(opt);
        }
        for (key, value) in &self.definitions {
            options.add_macro_definition(key, Some(value));
        }
        if self.flags.get_warnings_as_errors() {
            options.set_warnings_as_errors();
        }
        if self.flags.get_generate_debug_info() {
            options.set_generate_debug_info();
        }
        Ok(self.compiler.compile_into_spirv(
            &self.source, self.flags.get_shader_kind(),
            self.filename.as_ref().map_or("No Name", |v| v.as_str()),
            &self.entry_point, Some(&options))?)
    }

    /// Compiles the shader source code into SPIR-V bytecode and outputs a file with the specified
    /// filename and the .spv extension
    pub fn write_to_file<P: AsRef<Path>>(&mut self, out: P) -> Result<(), Box<dyn Error>> {
        let artifact = self.create_artifact()?;
        let file_out = out.as_ref().with_extension("spv");
        std::fs::write(file_out, artifact.as_binary_u8())?;
        Ok(())
    }

    /// Compiles the shader source code into SPIR-V bytecode and outputs a vector containing the bytecode.
    pub fn write_to_vec(&mut self) -> Result<Vec<u8>, Box<dyn Error>> {
        let artifact = self.create_artifact()?;
        Ok(artifact.as_binary_u8().to_vec())
    }
}