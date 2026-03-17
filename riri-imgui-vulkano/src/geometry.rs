use crate::error::Result;
use crate::resources::HasStandardMemoryAllocator;
use crate::vertex::{AppDrawData3D, AppDrawVert, AppVertex3D};
use glam::{Mat4, Vec2, Vec4};
use imgui::{DrawData, DrawIdx, DrawList};
use std::fmt::{Debug, Formatter};
use vulkano::buffer::{Buffer, BufferContents, BufferCreateInfo, BufferUsage, Subbuffer};
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryTypeFilter};

pub struct ImguiGeometryDraw<'a> {
    pub(crate) display_size: Vec2,
    pub(crate) display_pos: Vec2,
    pub(crate) clip_off: Vec2,
    pub(crate) clip_scale: Vec2,
    pub(crate) draw_lists: Vec<&'a DrawList>

}

impl<'a> Debug for ImguiGeometryDraw<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "ImguiGeometryDraw {{ display_size: {:?}, display_pos: {:?}, framebuffer_scale: {:?} }} ",
               self.display_size, self.display_pos, self.clip_scale)
    }
}

impl<'a> ImguiGeometryDraw<'a> {
    pub fn new(draw_data: &'a DrawData) -> Self {
        // Setup scale and translation:
        // Our visible imgui space lies from draw_data->DisplayPps (top left) to
        // draw_data->DisplayPos+data_data->DisplaySize (bottom right).
        // DisplayPos is (0,0) for single viewport apps.
        let display_size = Vec2::from(draw_data.display_size);
        let display_pos = Vec2::from(draw_data.display_pos);
        let clip_off = display_pos;
        let clip_scale = Vec2::from(draw_data.framebuffer_scale);
        Self {
            display_size,
            display_pos,
            clip_off,
            clip_scale,
            draw_lists: draw_data.draw_lists().collect()
        }
    }
}

#[derive(Debug)]
pub struct GeometryBufferBuilder;
impl GeometryBufferBuilder {
    pub(crate) fn from_iter<C, T>(
        data: Vec<T>,
        context: &C,
        usage: BufferUsage
    ) -> Result<Option<Subbuffer<[T]>>>
    where C: HasStandardMemoryAllocator,
          T: BufferContents {
        if data.len() == 0 { return Ok(None) }
        Ok(Some(Buffer::from_iter(
            context.allocator(),
            BufferCreateInfo {
                usage,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            data
        )?))
    }

    pub(crate) fn from_iter_generic<C, T, I>(
        data: I,
        context: &C,
        usage: BufferUsage
    ) -> Result<Option<Subbuffer<[T]>>>
    where C: HasStandardMemoryAllocator,
          T: BufferContents,
          I: IntoIterator<Item = T>,
          I::IntoIter: ExactSizeIterator
    {
        Ok(Some(Buffer::from_iter(
            context.allocator(),
            BufferCreateInfo {
                usage,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            data
        )?))
    }

    pub(crate) fn from_data<C, T>(
        data: T,
        context: &C,
        usage: BufferUsage
    ) -> Result<Option<Subbuffer<T>>>
    where C: HasStandardMemoryAllocator,
          T: BufferContents {
        Ok(Some(Buffer::from_data(
            context.allocator(),
            BufferCreateInfo {
                usage,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            data
        )?))
    }
}

#[derive(Debug)]
pub struct ImguiGeometry<'a> {
    pub(crate) vertex_buffer: Option<Subbuffer<[AppDrawVert]>>,
    pub(crate) index_buffer: Option<Subbuffer<[DrawIdx]>>,
    pub(crate) draw_data: Option<ImguiGeometryDraw<'a>>
}

impl<'a> ImguiGeometry<'a> {
    pub fn new<D>(
        dev: &D,
        draw_data: &'a DrawData
    ) -> Result<Self>
    where D: HasStandardMemoryAllocator {
        let mut vertices = Vec::with_capacity(draw_data.total_vtx_count as _);
        let mut indices = Vec::with_capacity(draw_data.total_idx_count as _);
        draw_data.draw_lists().for_each(|f| {
            let vtx_buffer = f.vtx_buffer();
            let app_vtx = unsafe { std::slice::from_raw_parts(
                vtx_buffer.as_ptr() as *const AppDrawVert, vtx_buffer.len()) };
            vertices.extend(app_vtx);
            indices.extend(f.idx_buffer());
        });
        let vertex_buffer = GeometryBufferBuilder::from_iter(
            vertices, dev, BufferUsage::VERTEX_BUFFER)?;
        let index_buffer = GeometryBufferBuilder::from_iter(
            indices, dev, BufferUsage::INDEX_BUFFER)?;
        let draw_data = Some(ImguiGeometryDraw::new(draw_data));
        Ok(Self { vertex_buffer, index_buffer, draw_data })
    }

    pub fn get_orthographic_projection(&self) -> Mat4 {
        match self.draw_data.as_ref() {
            Some(draw_data) => {
                let left = draw_data.display_pos.x;
                let right = draw_data.display_pos.x + draw_data.display_size.x;
                let top = draw_data.display_pos.y;
                let bottom = draw_data.display_pos.y + draw_data.display_size.y;
                Mat4::from_cols(
                    Vec4::new(2. / (right - left), 0., 0., 0.),
                    Vec4::new(0., 2. / (bottom - top), 0., 0.),
                    Vec4::new(0., 0., -1., 0.),
                    Vec4::new((right + left) / (left - right), (top + bottom) / (top - bottom), 0., 1.)
                )
            },
            None => Mat4::IDENTITY
        }
    }
}

impl<'a> Default for ImguiGeometry<'a> {
    fn default() -> Self {
        Self { vertex_buffer: None, index_buffer: None, draw_data: None }
    }
}

#[derive(Debug)]
pub struct BasicDrawGeometry {
    pub(crate) vertex_buffer: Option<Subbuffer<[AppVertex3D]>>,
    pub(crate) index_buffer: Option<Subbuffer<[u32]>>
}

impl BasicDrawGeometry {
    pub fn new<C: HasStandardMemoryAllocator>(context: &C, draw_data: &AppDrawData3D) -> Result<Self> {
        let mut vertices = Vec::with_capacity(draw_data.vertices.len() as _);
        let mut indices = Vec::with_capacity(draw_data.indices.len() as _);
        vertices.extend(draw_data.vertices.as_slice());
        indices.extend(draw_data.indices.as_slice());
        let vertex_buffer = GeometryBufferBuilder::from_iter(
            vertices, context, BufferUsage::VERTEX_BUFFER)?;
        let index_buffer = GeometryBufferBuilder::from_iter(
            indices, context, BufferUsage::INDEX_BUFFER)?;
        Ok(Self { vertex_buffer, index_buffer })
    }
}