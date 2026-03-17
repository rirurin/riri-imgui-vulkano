use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::ops::{Deref, DerefMut};
use std::path::Path;
use std::sync::Arc;
use riri_mod_tools_rt::logln;
use shaderc::ShaderKind;
use vulkano::shader::{EntryPoint, ShaderModule, ShaderModuleCreateInfo};
use riri_imgui_vulkano_shaders::AppCompiler;
use crate::error::{LibError, Result};
use crate::resources::HasLogicalDevice;

pub trait ShaderRegistry {
    /// Try to get a shader from the registry with the given name.
    fn get(&self, key: &str) -> Option<&AppShader>;
    /// Try to get references for the vertex and pixel shaders.
    /// This will error if your app doesn't load these shaders first.
    fn try_get_vertex_pixel(&self, vertex: &str, pixel: &str) -> Result<(&AppShader, &AppShader)>;
    /// Add a pixel (fragment) shader into the registry.
    fn add_pixel_shader<T: HasLogicalDevice, P: AsRef<Path>>(&mut self, object: &T, path: P) -> Result<()>;
    /// Add a vertex shader into the registry.
    fn add_vertex_shader<T: HasLogicalDevice, P: AsRef<Path>>(&mut self, object: &T, path: P) -> Result<()>;
    /// Add a compute shader into the registry.
    fn add_compute_shader<T: HasLogicalDevice, P: AsRef<Path>>(&mut self, object: &T, path: P) -> Result<()>;
    /// Add a geometry shader into the registry.
    fn add_geometry_shader<T: HasLogicalDevice, P: AsRef<Path>>(&mut self, object: &T, path: P) -> Result<()>;
}

#[macro_export]
macro_rules! try_get_vertex_pixel {
    ($registry:ident, $name:literal) => {
        $registry.try_get_vertex_pixel(concat!($name, ".vs"), concat!($name, ".ps"))
    }
}

#[derive(Debug)]
#[repr(transparent)]
pub struct LibShaderRegistry(HashMap<String, AppShader>);

impl Default for LibShaderRegistry {
    fn default() -> Self {
        Self(HashMap::new())
    }
}

impl Deref for LibShaderRegistry {
    type Target = HashMap<String, AppShader>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for LibShaderRegistry {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl ShaderRegistry for LibShaderRegistry {
    fn get(&self, key: &str) -> Option<&AppShader> {
        self.0.get(key)
    }

    fn try_get_vertex_pixel(&self, vertex: &str, pixel: &str) -> Result<(&AppShader, &AppShader)> {
        Ok((
            self.get(vertex).ok_or(LibError::CouldNotFindShader(vertex.to_owned()))?,
            self.get(pixel).ok_or(LibError::CouldNotFindShader(pixel.to_owned()))?,
        ))
    }

    fn add_pixel_shader<T: HasLogicalDevice, P: AsRef<Path>>(&mut self, object: &T, path: P) -> Result<()> {
        self.add_shader_inner(object, path, ShaderKind::Fragment, "main")
    }

    fn add_vertex_shader<T: HasLogicalDevice, P: AsRef<Path>>(&mut self, object: &T, path: P) -> Result<()> {
        self.add_shader_inner(object, path, ShaderKind::Vertex, "main")
    }

    fn add_compute_shader<T: HasLogicalDevice, P: AsRef<Path>>(&mut self, object: &T, path: P) -> Result<()> {
        self.add_shader_inner(object, path, ShaderKind::Compute, "main")
    }

    fn add_geometry_shader<T: HasLogicalDevice, P: AsRef<Path>>(&mut self, object: &T, path: P) -> Result<()> {
        self.add_shader_inner(object, path, ShaderKind::Geometry, "main")
    }
}

impl LibShaderRegistry {
    fn add_shader_inner<
        T: HasLogicalDevice,
        P: AsRef<Path>
    >(
        &mut self,
        context: &T,
        path: P,
        kind: ShaderKind,
        entry_point: &str,
    ) -> Result<()> {
        // Try getting SPIR-V bytecode first
        let shader_kind = path.as_ref().extension()
            .ok_or(LibError::NoFileExtensionOnShader)?.to_str().unwrap();
        let bytecode_ext = format!("{}.spv", shader_kind);
        let source_ext = format!("{}.glsl", shader_kind);
        let bytecode = path.as_ref().with_extension(&bytecode_ext);
        let source = path.as_ref().with_extension(&source_ext);
        let bytecode_exists = std::fs::exists(bytecode.as_path())?;
        let source_exists = std::fs::exists(source.as_path())?;
        if !bytecode_exists && source_exists {
            AppCompiler::from_path(source.as_path())?
                .set_shader_kind(kind)?
                .write_to_file(bytecode.as_path())?;
        }
        let shader = if source_exists || bytecode_exists {
            let filename = path.as_ref()
                .file_name().and_then(|v| v.to_str())
                .map(|v| v.to_string()).unwrap_or("No Name".to_string());
            let mut bytecode = unsafe {
                std::mem::transmute::<_, Vec<u32>>(std::fs::read(bytecode.as_path())?) };
            if bytecode.len() % 4 != 0 {
                return Err(Box::new(LibError::InvalidFileSizeForSpirvBytecode(bytecode.len())));
            }
            unsafe { bytecode.set_len(bytecode.len() / 4) };
            Some(AppShader::from_bytecode(context, bytecode, Some(filename), entry_point.to_string())?)
        } else {
            None
        }.ok_or(LibError::CouldNotFindShader(path.as_ref().to_str().unwrap().to_string()))?;
        logln!(Information, "Added shader \"{}\" into the registry.", shader.filename());
        self.insert(shader.filename().to_string(), shader);
        Ok(())
    }
}

pub struct AppShader {
    pub code: Vec<u32>,
    filename: String,
    entry_point: String,
    pub module: Option<Arc<ShaderModule>>
}

impl Debug for AppShader {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "AppShader {{ }}")
    }
}

impl AppShader {
    pub fn from_source<T: HasLogicalDevice>(context: &T, compiler: AppCompiler<'_>) -> Result<Self> {
        let artifact = compiler.create_artifact()?;
        let mut out = Self {
            code: artifact.as_binary().to_vec(),
            filename: compiler.get_filename().unwrap().to_string(),
            entry_point: compiler.get_entry_point().to_string(),
            module: None };
        out.module = Some(unsafe { ShaderModule::new(
            context.logical_device(),
            ShaderModuleCreateInfo::new(out.code.as_slice()))?
        });
        Ok(out)
    }

    pub fn from_bytecode<T: HasLogicalDevice>(
        context: &T, code: Vec<u32>, filename: Option<String>, entry_point: String
    ) -> Result<Self> {
        let filename = filename.unwrap_or("No Name".to_string());
        let mut out = Self { code, filename, entry_point, module: None };
        out.module = Some(unsafe { ShaderModule::new(
            context.logical_device(),
            ShaderModuleCreateInfo::new(out.code.as_slice()))?
        });
        Ok(out)
    }

    pub fn filename(&self) -> &str {
        &self.filename
    }

    pub fn entry_point(&self) -> EntryPoint {
        self.module.as_ref().unwrap().entry_point(&self.entry_point).unwrap()
    }
}