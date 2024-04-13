//Definition of vertex binding and attributes
//Every vertex is supposed to have position, color, normal and texture uv

use std::{hash::{Hasher, Hash}, mem};

use ash::vk::{Format, VertexInputAttributeDescription, VertexInputBindingDescription, VertexInputRate};

#[derive(Copy, Clone)]
pub struct VertexData {
    vertex_position: glm::Vec3,
    vertex_normal: glm::Vec3,
    texture_uv: glm::Vec2
}

pub struct VertexInput {
    pub vertex_data: Vec<VertexData>,
}

impl VertexData {
    pub fn new(vertex_position: glm::Vec3, vertex_normal: glm::Vec3, texture_uv: glm::Vec2) -> Self {
        Self {
            vertex_position,
            vertex_normal,
            texture_uv
        }
    }
}

impl PartialEq for VertexData {
    fn eq(&self, other: &Self) -> bool {
        self.vertex_position == other.vertex_position && self.vertex_normal == other.vertex_normal && self.texture_uv == other.texture_uv
    }
}

impl Hash for VertexData {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.vertex_position[0].to_bits().hash(state);
        self.vertex_position[1].to_bits().hash(state);
        self.vertex_position[2].to_bits().hash(state);
        self.vertex_normal[0].to_bits().hash(state);
        self.vertex_normal[1].to_bits().hash(state);
        self.vertex_normal[2].to_bits().hash(state);
        self.texture_uv[0].to_bits().hash(state);
        self.texture_uv[1].to_bits().hash(state);
    }
}

impl Eq for VertexData {}

impl VertexInput {
    pub fn get_binding_descriptions() -> Vec<VertexInputBindingDescription> {
        let binding_description = VertexInputBindingDescription::builder()
            .binding(0)
            .stride(mem::size_of::<VertexData>() as u32)
            .input_rate(VertexInputRate::VERTEX)
            .build();

        let mut binding_descriptions = Vec::new();
        binding_descriptions.push(binding_description);

        binding_descriptions
    }

    pub fn get_attribute_descriptions() -> Vec<VertexInputAttributeDescription> {
        let position_attribute = VertexInputAttributeDescription::builder()
            .binding(0)
            .location(0)
            .format(Format::R32G32B32_SFLOAT)
            .offset(mem::offset_of!(VertexData, vertex_position) as u32)
            .build();

        let normal_attribute = VertexInputAttributeDescription::builder()
            .binding(0)
            .location(1)
            .format(Format::R32G32B32_SFLOAT)
            .offset(mem::offset_of!(VertexData, vertex_normal) as u32)
            .build();

        let texture_attribute = VertexInputAttributeDescription::builder()
            .binding(0)
            .location(2)
            .format(Format::R32G32_SFLOAT)
            .offset(mem::offset_of!(VertexData, texture_uv) as u32)
            .build();

        let mut attribute_descriptions = Vec::new();
        attribute_descriptions.push(position_attribute);
        attribute_descriptions.push(normal_attribute);
        attribute_descriptions.push(texture_attribute);

        attribute_descriptions
    }

    pub fn new() -> Self {
        let vertex_data: Vec<VertexData> = Vec::new();
        
        Self {
            vertex_data
        }
    }

    pub fn add_vertices(&mut self, vertex_data: &mut Vec<VertexData>) {
        self.vertex_data.append(vertex_data);
    }

    pub fn size(&self) -> usize {
        mem::size_of::<VertexData>() * self.vertex_data.len()
    }
}
