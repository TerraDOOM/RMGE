use std::{
    fs::File,
    io::{self, Read},
    mem::{self, ManuallyDrop},
    rc::Rc,
};

use byteorder::{BigEndian, LittleEndian, NativeEndian, ReadBytesExt};

use gfx_hal::{
    device::Device,
    image::Extent,
    pass::Subpass,
    pso::{
        AttributeDesc, BakedStates, BasePipeline, BlendDesc, BlendOp, BlendState, ColorBlendDesc,
        ColorMask, DepthStencilDesc, DepthTest, DescriptorSetLayoutBinding, EntryPoint, Face,
        Factor, FrontFace, GraphicsPipelineDesc, GraphicsShaderSet, InputAssemblerDesc, LogicOp,
        PipelineCreationFlags, PolygonMode, Primitive, Rasterizer, ShaderStageFlags,
        Specialization, State, StencilTest, VertexBufferDesc, Viewport,
    },
    Backend,
};

use crate::error::{Error, ShaderKind};

use log::error;

// helper function
fn snd<T, U>(a: (T, U)) -> U {
    a.1
}

static VERT_SHADER_FILENAME: &'static str = "vert.spv";
static FRAG_SHADER_FILENAME: &'static str = "frag.spv";

fn read_shader_data(mut f: File) -> Result<Vec<u32>, io::Error> {
    let file_len = f.metadata()?.len();

    if file_len % 4 != 0 {
        error!(target: "rmge", "shader file length is not divisible by 4!");
    }

    let mut v = Vec::new();
    v.resize_with(file_len as usize / 4, || 0);
    unsafe {}
    f.read_u32_into::<LittleEndian>(&mut v[..])?;
    Ok(v)
}

#[derive(Debug)]
pub struct PipelineData<B: Backend, D: Device<B>> {
    pub device: Rc<ManuallyDrop<D>>,
    pub pipeline_layout: ManuallyDrop<B::PipelineLayout>,
    pub graphics_pipeline: ManuallyDrop<B::GraphicsPipeline>,
    pub descriptor_set_layouts: Vec<B::DescriptorSetLayout>,
}

impl<B: Backend, D: Device<B>> PipelineData<B, D> {
    pub fn new(
        device: Rc<ManuallyDrop<D>>,
        extent: Extent,
        render_pass: &B::RenderPass,
        vertex_buffers: Vec<VertexBufferDesc>,
        attributes: Vec<AttributeDesc>,
    ) -> Result<Self, Error> {
        let frag_shader_file = File::open("frag.spv").map_err(|e| Error::IOError(e))?;
        let vert_shader_file = File::open("vert.spv").map_err(|e| Error::IOError(e))?;
        let vert_data = read_shader_data(vert_shader_file).map_err(|e| Error::IOError(e))?;
        let frag_data = read_shader_data(frag_shader_file).map_err(|e| Error::IOError(e))?;

        let bindings: &'static [DescriptorSetLayoutBinding] = &[];
        let immutable_samplers: &'static [B::Sampler] = &[];
        let push_constants: &'static [(ShaderStageFlags, core::ops::Range<u32>)] = &[];

        let mut pipeline_builder = PipelineBuilder::new(device.clone());
        let (vert_shader_module, frag_shader_module, descriptor_set_layouts, layout) =
            pipeline_builder
                .add_vert_shader(&vert_data)
                .map_err(snd)?
                .add_frag_shader(&frag_data)
                .map_err(snd)?
                .add_descriptor_set_layout(&bindings, &immutable_samplers)
                .map_err(snd)?
                .add_pipeline_layout(&push_constants)
                .map_err(snd)?
                .into_data()
                .map_err(|_| Error::PipelineCreation)?;

        let (vs_entry, fs_entry): (EntryPoint<'_, B>, EntryPoint<'_, B>) = (
            EntryPoint {
                entry: "main",
                module: &vert_shader_module,
                specialization: Specialization::EMPTY,
            },
            EntryPoint {
                entry: "main",
                module: &frag_shader_module,
                specialization: Specialization::EMPTY,
            },
        );

        let shaders = GraphicsShaderSet {
            vertex: vs_entry,
            hull: None,
            domain: None,
            geometry: None,
            fragment: Some(fs_entry),
        };

        let rasterizer = Rasterizer {
            depth_clamping: false,
            polygon_mode: PolygonMode::Fill,
            cull_face: Face::NONE,
            front_face: FrontFace::Clockwise,
            depth_bias: None,
            conservative: false,
            line_width: State::Static(1.0),
        };

        let depth_stencil = DepthStencilDesc {
            depth: None,
            depth_bounds: false,
            stencil: None,
        };

        let blender = {
            let blend_state = BlendState {
                color: BlendOp::Add {
                    src: Factor::One,
                    dst: Factor::Zero,
                },
                alpha: BlendOp::Add {
                    src: Factor::One,
                    dst: Factor::Zero,
                },
            };
            BlendDesc {
                logic_op: Some(LogicOp::Copy),
                targets: vec![ColorBlendDesc {
                    mask: ColorMask::ALL,
                    blend: Some(blend_state),
                }],
            }
        };

        let baked_states = BakedStates {
            viewport: Some(Viewport {
                rect: extent.rect(),
                depth: (0.0..1.0),
            }),
            scissor: Some(extent.rect()),
            blend_color: None,
            depth_bounds: None,
        };

        let input_assembler = InputAssemblerDesc::new(Primitive::TriangleList);

        let gfx_pipeline = {
            let desc = GraphicsPipelineDesc {
                shaders,
                rasterizer,
                vertex_buffers,
                attributes,
                input_assembler,
                blender,
                depth_stencil,
                multisampling: None,
                baked_states,
                layout: &layout,
                subpass: Subpass {
                    index: 0,
                    main_pass: &render_pass,
                },
                flags: PipelineCreationFlags::empty(),
                parent: BasePipeline::None,
            };

            unsafe { device.create_graphics_pipeline(&desc, None) }
        };

        unsafe {
            device.destroy_shader_module(vert_shader_module);
            device.destroy_shader_module(frag_shader_module);
        }

        match gfx_pipeline {
            Ok(graphics_pipeline) => Ok(Self {
                device,
                pipeline_layout: ManuallyDrop::new(layout),
                graphics_pipeline: ManuallyDrop::new(graphics_pipeline),
                descriptor_set_layouts,
            }),
            Err(_) => {
                unsafe {
                    device.destroy_pipeline_layout(layout);
                    for d_layout in descriptor_set_layouts {
                        device.destroy_descriptor_set_layout(d_layout);
                    }
                }
                Err(Error::PipelineCreation)
            }
        }
    }
}

pub struct PipelineBuilder<'a, B: Backend, D: Device<B>> {
    device: Rc<ManuallyDrop<D>>,
    frag_shader: Option<B::ShaderModule>,
    vert_shader: Option<B::ShaderModule>,
    push_consts: Option<&'a [(ShaderStageFlags, core::ops::Range<u32>)]>,
    descriptor_set_layouts: Vec<B::DescriptorSetLayout>,
    pipeline_layout: Option<B::PipelineLayout>,
}

impl<'a, B: Backend, D: Device<B>> PipelineBuilder<'a, B, D> {
    pub fn new(device: Rc<ManuallyDrop<D>>) -> Self {
        Self {
            device,
            push_consts: None,
            frag_shader: None,
            vert_shader: None,
            descriptor_set_layouts: vec![],
            pipeline_layout: None,
        }
    }

    pub fn add_vert_shader(mut self, vert_data: &[u32]) -> Result<Self, (Self, Error)> {
        if let Some(old) = mem::replace(
            &mut self.vert_shader,
            Some(unsafe {
                match self.device.create_shader_module(vert_data) {
                    Ok(m) => m,
                    Err(e) => return Err((self, Error::ShaderCreation(ShaderKind::Vertex, e))),
                }
            }),
        ) {
            unsafe {
                self.device.destroy_shader_module(old);
            }
        }

        Ok(self)
    }

    pub fn add_frag_shader(mut self, frag_data: &[u32]) -> Result<Self, (Self, Error)> {
        if let Some(old) = mem::replace(
            &mut self.frag_shader,
            Some(unsafe {
                match self.device.create_shader_module(frag_data) {
                    Ok(m) => m,
                    Err(e) => return Err((self, Error::ShaderCreation(ShaderKind::Fragment, e))),
                }
            }),
        ) {
            unsafe {
                self.device.destroy_shader_module(old);
            }
        }
        Ok(self)
    }

    pub fn add_descriptor_set_layout(
        mut self,
        bindings: &[DescriptorSetLayoutBinding],
        immutable_samplers: &[B::Sampler],
    ) -> Result<Self, (Self, Error)> {
        self.descriptor_set_layouts.push(unsafe {
            match self
                .device
                .create_descriptor_set_layout(bindings, immutable_samplers)
            {
                Ok(layout) => layout,
                Err(_) => return Err((self, Error::DescriptorSetLayoutCreation)),
            }
        });

        Ok(self)
    }

    pub fn add_pipeline_layout(
        mut self,
        push_constants: &'a [(ShaderStageFlags, core::ops::Range<u32>)],
    ) -> Result<Self, (Self, Error)> {
        if let Some(old) = mem::replace(
            &mut self.pipeline_layout,
            Some(unsafe {
                match self
                    .device
                    .create_pipeline_layout(&self.descriptor_set_layouts, push_constants)
                {
                    Ok(layout) => layout,
                    Err(_) => return Err((self, Error::PipelineLayoutCreation)),
                }
            }),
        ) {
            unsafe {
                self.device.destroy_pipeline_layout(old);
            }
        }
        self.push_consts = Some(push_constants);
        Ok(self)
    }

    pub fn into_data(
        mut self,
    ) -> Result<
        (
            B::ShaderModule,
            B::ShaderModule,
            Vec<B::DescriptorSetLayout>,
            B::PipelineLayout,
        ),
        Self,
    > {
        if self.vert_shader.is_some()
            && self.frag_shader.is_some()
            && self.pipeline_layout.is_some()
        {
            if let (
                Some(vert_shader),
                Some(frag_shader),
                descriptor_set_layouts,
                Some(pipeline_layout),
            ) = (
                mem::replace(&mut self.vert_shader, None),
                mem::replace(&mut self.frag_shader, None),
                mem::replace(&mut self.descriptor_set_layouts, Vec::new()),
                mem::replace(&mut self.pipeline_layout, None),
            ) {
                Ok((
                    vert_shader,
                    frag_shader,
                    descriptor_set_layouts,
                    pipeline_layout,
                ))
            } else {
                unreachable!()
            }
        } else {
            Err(self)
        }
    }
}

impl<B: Backend, D: Device<B>> std::ops::Drop for PipelineData<B, D> {
    fn drop(&mut self) {
        unsafe {
            use std::ptr::read;
            self.device
                .destroy_pipeline_layout(ManuallyDrop::into_inner(read(&self.pipeline_layout)));
            self.device
                .destroy_graphics_pipeline(ManuallyDrop::into_inner(read(&self.graphics_pipeline)));
            for layout in self.descriptor_set_layouts.drain(..) {
                self.device.destroy_descriptor_set_layout(layout);
            }
        }
    }
}

impl<'a, B: Backend, D: Device<B>> std::ops::Drop for PipelineBuilder<'a, B, D> {
    fn drop(&mut self) {
        unsafe {
            if let Some(module) = mem::replace(&mut self.vert_shader, None) {
                self.device.destroy_shader_module(module);
            }

            if let Some(module) = mem::replace(&mut self.frag_shader, None) {
                self.device.destroy_shader_module(module);
            }

            for descriptor_set_layout in self.descriptor_set_layouts.drain(..) {
                self.device
                    .destroy_descriptor_set_layout(descriptor_set_layout);
            }

            if let Some(layout) = mem::replace(&mut self.pipeline_layout, None) {
                self.device.destroy_pipeline_layout(layout);
            }
        }
    }
}
