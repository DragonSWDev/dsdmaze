//Graphics pipeline creation

use std::ffi::CStr;

use ash::{vk::{self, ColorComponentFlags, CompareOp, CullModeFlags, DynamicState, FrontFace, GraphicsPipelineCreateInfo, LogicOp, Pipeline, PipelineColorBlendAttachmentState, PipelineColorBlendStateCreateInfo, 
    PipelineDepthStencilStateCreateInfo, PipelineDynamicStateCreateInfo, PipelineInputAssemblyStateCreateInfo, PipelineLayout, PipelineMultisampleStateCreateInfo, PipelineRasterizationStateCreateInfo, 
    PipelineShaderStageCreateInfo, PipelineVertexInputStateCreateInfo, PipelineViewportStateCreateInfo, PolygonMode, PrimitiveTopology, RenderPass, SampleCountFlags, ShaderModule, ShaderStageFlags, 
    VertexInputAttributeDescription, VertexInputBindingDescription}, Device};

pub struct VulkanPipeline {
    shader_stages: Vec<PipelineShaderStageCreateInfo>,
    topology: PrimitiveTopology,
    vertex_input_bindings: Vec<VertexInputBindingDescription>,
    vertex_input_attributes: Vec<VertexInputAttributeDescription>
}

impl VulkanPipeline {
    pub fn new(topology: PrimitiveTopology) -> VulkanPipeline {        
        Self {
            shader_stages: Vec::new(),
            topology,
            vertex_input_bindings: Vec::new(),
            vertex_input_attributes: Vec::new()
        }
    }

    pub fn build_pipeline(&mut self, logical_device: &Device, pipeline_layout: PipelineLayout, render_pass: RenderPass, sample_count: SampleCountFlags) -> Pipeline {
        if self.shader_stages.is_empty() {
            panic!("Attempted to build pipeline without shader stages.");
        }

        let input_assembly = PipelineInputAssemblyStateCreateInfo::builder()
            .topology(self.topology)
            .primitive_restart_enable(false);

        let rasterization_state = PipelineRasterizationStateCreateInfo::builder()
            .depth_clamp_enable(false)
            .rasterizer_discard_enable(false)
            .polygon_mode(PolygonMode::FILL)
            .line_width(1.0)
            .cull_mode(CullModeFlags::NONE)
            .front_face(FrontFace::CLOCKWISE)
            .depth_bias_enable(false)
            .depth_bias_constant_factor(0.0)
            .depth_bias_clamp(0.0)
            .depth_bias_constant_factor(0.0);

        let multisample_state = PipelineMultisampleStateCreateInfo::builder()
            .sample_shading_enable(true)
            .rasterization_samples(sample_count)
            .min_sample_shading(0.4)
            .alpha_to_coverage_enable(false)
            .alpha_to_one_enable(false);

        let color_blend_attachment_state = PipelineColorBlendAttachmentState::builder()
            .color_write_mask(ColorComponentFlags::R | ColorComponentFlags::G | ColorComponentFlags::B | ColorComponentFlags::A)
            .blend_enable(false)
            .build();

        let pipeline_dynamic_states = PipelineDynamicStateCreateInfo::builder()
            .dynamic_states(&[DynamicState::VIEWPORT, DynamicState::SCISSOR]);

        let pipeline_viewport_state = PipelineViewportStateCreateInfo::builder()
            .viewport_count(1)
            .scissor_count(1);

        let pipeline_color_blend_state = PipelineColorBlendStateCreateInfo::builder()
            .logic_op_enable(false)
            .logic_op(LogicOp::COPY)
            .attachments(std::slice::from_ref(&color_blend_attachment_state));

        let pipeline_depth_stencil_state = PipelineDepthStencilStateCreateInfo::builder()
            .depth_test_enable(true)
            .depth_write_enable(true)
            .depth_compare_op(CompareOp::LESS)
            .depth_bounds_test_enable(false)
            .stencil_test_enable(false);

        let vertex_input_info = PipelineVertexInputStateCreateInfo::builder()
            .vertex_binding_descriptions(&self.vertex_input_bindings[..])
            .vertex_attribute_descriptions(&self.vertex_input_attributes[..]);

        let pipeline_create_info = GraphicsPipelineCreateInfo::builder()
            .stages(&self.shader_stages)
            .input_assembly_state(&input_assembly)
            .vertex_input_state(&vertex_input_info)
            .dynamic_state(&pipeline_dynamic_states)
            .viewport_state(&pipeline_viewport_state)
            .rasterization_state(&rasterization_state)
            .multisample_state(&multisample_state)
            .color_blend_state(&pipeline_color_blend_state)
            .depth_stencil_state(&pipeline_depth_stencil_state)
            .layout(pipeline_layout)
            .render_pass(render_pass)
            .subpass(0);

        let graphics_pipeline = unsafe {
            logical_device.create_graphics_pipelines(vk::PipelineCache::null(), &[pipeline_create_info.build()], None).expect("Graphics pipeline creation failed.")
        };

        graphics_pipeline[0]
    }

    pub fn add_shader_stage(&mut self, stage: ShaderStageFlags, shader_module: ShaderModule) {
        let shader_name = unsafe {
            CStr::from_bytes_with_nul_unchecked(b"main\0")
        };

        let shader_stage_info = PipelineShaderStageCreateInfo::builder()
            .stage(stage)
            .module(shader_module)
            .name(shader_name)
            .build();

        self.shader_stages.push(shader_stage_info);
    }

    pub fn add_vertex_input_bindings(&mut self, bindings: &mut Vec<VertexInputBindingDescription>) {
        self.vertex_input_bindings.append(bindings);
    }

    pub fn add_vertex_input_attributes(&mut self, attributes: &mut Vec<VertexInputAttributeDescription>) {
        self.vertex_input_attributes.append(attributes);
    }
}
