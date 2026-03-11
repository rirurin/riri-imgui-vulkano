use std::alloc::Layout;
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::ptr::NonNull;
use glam::{U8Vec4, Vec2, Vec3};
use imgui::DrawVert;
use vulkano::buffer::{BufferContents, BufferContentsLayout};
use vulkano::format::Format;
use vulkano::pipeline::graphics::vertex_input::{Vertex, VertexBufferDescription, VertexInputRate, VertexMemberInfo};

/// Thin wrapper over imgui-rs DrawVert type.
/// vulkano Vertex types were manually implemented based off the macro generation code
#[derive(Debug, Copy, Clone, PartialEq)]
#[repr(transparent)]
pub struct AppDrawVert(pub(crate) DrawVert);

type PosType = [f32; 2];
type UVType = [f32; 2];
type ColType = [u8; 4];

impl Deref for AppDrawVert {
    type Target = DrawVert;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for AppDrawVert {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

unsafe impl BufferContents for AppDrawVert {
    const LAYOUT: BufferContentsLayout = BufferContentsLayout::from_field_layouts(&[
        Layout::new::<PosType>(),
        Layout::new::<UVType>(),
    ], <ColType as BufferContents>::LAYOUT);

    unsafe fn ptr_from_slice(slice: NonNull<[u8]>) -> *mut Self {
        let data = <*mut [u8]>::cast::<u8>(slice.as_ptr());
        let head_size = <Self as BufferContents>::LAYOUT.head_size() as usize;
        let element_size = <Self as BufferContents>::LAYOUT.element_size().unwrap_or(1) as usize;
        debug_assert!(slice.len() >= head_size);
        // For unsized types
        let tail_size = slice.len() - head_size;
        debug_assert!(tail_size % element_size == 0);
        unsafe { <ColType as BufferContents>::ptr_from_slice(
            NonNull::new_unchecked(std::ptr::slice_from_raw_parts_mut(
                data.add(head_size),
                tail_size
            ))
        ).byte_sub(head_size) as *mut Self }
    }
}

unsafe impl Vertex for AppDrawVert {
    #[inline(always)]
    fn per_vertex() -> VertexBufferDescription {
        let mut builder = CreateVertexMembers::default();
        let mut members = HashMap::new();
        members.insert("pos".to_string(), builder.create::<PosType>(Format::R32G32_SFLOAT));
        members.insert("uv".to_string(), builder.create::<UVType>(Format::R32G32_SFLOAT));
        members.insert("col".to_string(), builder.create::<ColType>(Format::R8G8B8A8_UNORM));

        VertexBufferDescription {
            members,
            stride: size_of::<Self>() as u32,
            input_rate: VertexInputRate::Vertex
        }
    }

    #[inline(always)]
    fn per_instance() -> VertexBufferDescription {
        Self::per_vertex().per_instance()
    }

    #[inline(always)]
    fn per_instance_with_divisor(divisor: u32) -> VertexBufferDescription {
        Self::per_vertex().per_instance_with_divisor(divisor)
    }
}

#[derive(Debug)]
struct CreateVertexMembers {
    offset: usize
}

impl Default for CreateVertexMembers {
    fn default() -> Self {
        Self { offset: 0 }
    }
}

impl CreateVertexMembers {
    pub fn create<T>(&mut self, format: Format) -> VertexMemberInfo {
        assert_eq!(size_of::<T>(), format.block_size() as usize);
        let block_size: u32 = format.block_size().try_into().unwrap();
        let out = VertexMemberInfo {
            offset: self.offset.try_into().unwrap(),
            format,
            num_elements: size_of::<T>() as u32 / block_size,
            stride: block_size
        };
        self.offset += size_of::<T>();
        out
    }
}

#[derive(Debug, BufferContents, Vertex)]
#[repr(C)]
pub struct AppVertex3D {
    #[format(R32G32B32_SFLOAT)]
    pub(crate) pos: [f32; 3],
    #[format(R32G32B32_SFLOAT)]
    pub(crate) nrm: [f32; 3],
    #[format(R8G8B8A8_UINT)]
    pub(crate) col: [u8; 4],
    #[format(R32G32_SFLOAT)]
    pub(crate) uv: [f32; 2]
}

impl AppVertex3D {
    pub fn pos(pos: Vec3) -> Self {
        Self::pos_color_uv(pos, U8Vec4::MAX, Vec2::ZERO)
    }
    pub fn pos_color(pos: Vec3, color: U8Vec4) -> Self {
        Self::pos_color_uv(pos, color, Vec2::ZERO)
    }
    pub fn pos_uv(pos: Vec3, uv: Vec2) -> Self {
        Self::pos_color_uv(pos, U8Vec4::MAX, uv)
    }
    pub fn pos_color_uv(pos: Vec3, color: U8Vec4, uv: Vec2) -> Self {
        Self::new(pos, Vec3::ZERO, color, uv)
    }
    pub fn new(pos: Vec3, nrm: Vec3, color: U8Vec4, uv: Vec2) -> Self {
        Self { pos: pos.to_array(), nrm: nrm.to_array(), col: color.to_array(), uv: uv.to_array() }
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct AppDrawData3D {
    pub(crate) vertices: Vec<AppVertex3D>,
    pub(crate) indices: Vec<u32>,
}

impl AppDrawData3D {
    pub fn get_vertices(&self) -> &[AppVertex3D] { &self.vertices }
    pub fn get_indices(&self) -> &[u32] { &self.indices }

    pub const fn new(vertices: Vec<AppVertex3D>, indices: Vec<u32>) -> Self {
        Self { vertices, indices }
    }
    pub const fn empty() -> Self {
        Self { vertices: vec![], indices: vec![] }
    }
}