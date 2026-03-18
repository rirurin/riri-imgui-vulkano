# riri-imgui-vulkano

## Usage

A Rust crate for building cross-platform real-time GUI applications using imgui-rs and vulkano.

This was designed as a rewrite for an existing GUI stack I've used for some applications to use non-Windows specific APIs (specifically migrating from Win32 + Direct3D11 to winit + Vulkan) to allow for Linux native builds of these apps.

### riri-imgui-vulkano-app

A complete example of an app built on top of `riri-imgui-vulkano`, featuring a rendering pass with two subpasses for 3D vertices and UI. The example includes controller support using [glirs-imgui-support](github.com/rirurin/gilrs-imgui-support) to move the camera around and custom fonts.

![A preview of the app, with the imgui demo window and a spinning cube.](assets/app_preview.gif)

### riri-imgui-vulkano

A crate containing a collection of (mostly) reusable Vulkan and Imgui objects, with the aim of simplifying Vulkan's initialization and rendering for app developers and to provide customizability for these objects with traits.

#### `RendererContext`

A wrapper for several Vulkan objects created at the beginning of the program and exist for it's lifetime. This selects the target devices, builds the window surface and creates the debug messenger. This is initialized using `new`:

```rust
pub fn new<D: HasDisplayHandle>(display_handle: D, window: Arc<Box<dyn Window>>, app_name: Option<String>) -> Result<Self>;
```

*Note that this uses winit 0.31.0 beta 2 (see [Dependencies](#dependencies)) so the API is different from what it is with the current stable version of winit as of writing this (0.30.13)*

In the example app, this is created when `ApplicationHandler::can_create_surfaces` is called, and is passed as a parameter into the app's `VulkanContext`, used to store all the objects from the library crate:

```rust
// ...
let lib_ctx = RendererContext::new(event_loop, self.get_window(), Some(self.get_name().to_string())).unwrap();
let app_ctx = VulkanContext::new(lib_ctx, self.get_window(), self.get_imgui_mut());
// ...
```

#### Swapchains (`SwapchainImpl`, `BaseSwapchain`)

The swapchain object `BaseSwapchain` stores the swapchain, framebuffers and synchornization objects for async command buffer execution.
Apps are expected to implement `SwapchainImpl`, which require defining methods for `make_framebuffer` and `refresh`:

```rust
// set_framebuffers calls this for each framebuffer
fn make_framebuffer<R: HasRenderPass>(&self, image: Arc<Image>, render_pass: &R) -> Result<Arc<Framebuffer>>;
fn refresh<
    T0: HasStandardMemoryAllocator,
    T1: HasRenderPass
>(&mut self, context: &T0, render_pass: &T1, extent: UVec2) -> Result<()>;
```

This allows for apps to freely pass the attachments that their render pass needs into each framebuffer and to ensure that those attachments are updated when the window is resized. The example app `AppSwapchain` uses this to add a depth stencil attachment on top of the base swapchain.

#### Render Passes (`LibRenderPass`, `BaseRenderPass`, `ImguiRenderPass`, `Basic3dRenderPass`)

The render pass objects, such as `ImguiRenderPass` and `Basic3dRenderPass` are builders for constructing a `LibRenderPass` (a thin wrapper over `Arc<RenderPass>`) which is formatted as a single subpass for rendering UI or basic 3D vertices.
These objects implement `RenderPassBuilder` to provide a `build` to allow for the conversion.

The example app uses `RenderPassBuilder` to build a render pass with two subpasses to combine 3D geometry and UI drawing.

#### Descriptor Registry (`LibDescriptorSets`)

Stores a map of all registered descriptor sets based off their memory address (defined as a `TextureId`).

There are some helper objects for writing particular descriptor sets such as Imgui's fonts (`ImguiFontBuilder`).

#### Shader Registry (`LibShaderRegistry`)

Stores a map of all registered shaders based off their name. `LibShaderRegistry` supports compiling shaders on-the-fly, although it's best to compile shaders before hand to prevent freezing.

#### Graphics Pipelines (`CreateGraphicsPipeline`, `ImguiGraphicsPipeline`, `Basic3dGraphicsPipeline`)

Contains some graphics pipeline presets to define how drawing basic 3D geometry and imgui's UI will be drawn. App developers can define their own pipelines using `CreateGraphicsPipeline` which requires a single `new` function:

```rust
fn new<
    T0: HasLogicalDevice,
    T1: ShaderRegistry,
    T2: HasRenderPass
>(
    device: &T0,
    viewport: &Viewport,
    scissor: &Scissor,
    shaders: &T1,
    render_pass: &T2
) -> Result<Self>;
```

`ImguiGraphicsPipeline` and `Basic3dGraphicsPipeline` use a constant type parameter to indicate which subpass it belongs to in the render pass. The example app renders 3D geometry first, so it defines it's pipeline as:

```rust
let pipeline = AppPipeline::new(
    Basic3dGraphicsPipeline::<0>::new(
        &context, &viewport, &scissor, &shaders, &render_pass)?,
    ImguiGraphicsPipeline::<1>::new(
        &context, &viewport, &scissor, &shaders, &render_pass)?,
);
```

#### Command Lists

The command buffer building process is split into a collection of objects, starting with the initialization of `GpuCommandBuilder`, which sets the command allocator and command buffer usage (`GpuCommandUsageOnce`, `GpuCommandUsageMultiple` or `GpuCommandUsageAsync`):

```rust
let mut builder: GpuCommandBuilder<_, GpuCommandUsageOnce>
    = GpuCommandBuilder::new(&allocator, context)?;
```

From there, building a command list consists of calling `GpuCommandSet::build` for structs that implement `GpuCommandSet`. The list of builtins are:

- **CopyBufferToImage** - used for uploading texture data to an Image.
- **CopyImageToBuffer** - used to retrieve image data so it's CPU-accessible.
- **StartRenderPass** - starts a render pass with the given framebuffer and clear values if the render pass LoadOp is Clear.
- **NextSubpass** - move to the next subpass in the current render pass.
- **EndRenderPass** - ends it
- **DrawImgui** - given Imgui geometry, it will be drawn onto the output attachments.
- **DrawBasic3d** - given basic 3D geometry, it will be drawn onto the output attachments.

### riri-imgui-vulkano-shaders

A separate crate to handle shader compilation so it can be utilized in build scripts. See build.rs in `riri-imgui-vulkano-app` for an example.

The naming convention used for shaders is `[shader name].[shader type].[file format]`. Shader type refers to the stage that the shader is applicable to, which can be **.vs** (vertex), **.ps** (pixel/fragment), **.cs** (compute) and **.gs** (geometry.) The file format is either **.glsl** for shader source or **.spv** for SPIR-V bytecode produced by the compiler.

*(.hlsl has some quirks which cause issues when building vertex buffer descriptions, `#[derive(Vertex)]` is broken for example. Will be something to investigate later.)*

A standalone shader compiler is available in `examples/shader-compiler.rs` which requires files to follow the namign above.

## Dependencies

This crate was designed to work with my customized GUI stack:
- [gilrs-imgui-support](github.com/rirurin/gilrs-imgui-support), used by the sample app.
- [imgui-rs](https://github.com/rirurin/imgui-rs), based on imgui 1.91.3
- [imgui-winit-support](https://github.com/rirurin/imgui-winit-support) to add support for winit 0.31.0
- [riri-mod-tools](https://github.com/rirurin/riri-mod-tools) for the logger
- [winit](https://github.com/rirurin/winit), based on winit 0.31.0 beta 2 (this has some API changes from 0.30.x)

