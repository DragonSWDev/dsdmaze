#version 450 core

layout (push_constant) uniform constants
{
	mat4 model_matrix;
    int texture_index;
} pcs;

layout (binding = 0) uniform UniformBufferObject {
    mat4 view_matrix;
    mat4 projection_matrix;
    vec3 light_positon;
    vec3 light_color;
} ubo;

layout (location = 0) in vec3 aVertexPosition;
layout (location = 1) in vec3 aNormalAttribute;
layout (location = 2) in vec2 aTexturePosition;

layout (location = 0) out vec2 textureCoords;
layout (location = 1) out vec3 normalVector;
layout (location = 2) out vec3 fragmentPosition;
layout (location = 3) out vec3 lightPosition;
layout (location = 4) out vec3 lightColor;
layout (location = 5) out flat int textureIndex;

void main()
{
    textureCoords = aTexturePosition;
    normalVector = mat3(transpose(inverse(ubo.view_matrix * pcs.model_matrix))) * aNormalAttribute;

    //Change to view space before sending to fragment shader
    fragmentPosition = vec3(ubo.view_matrix * pcs.model_matrix * vec4(aVertexPosition, 1.0));
    lightPosition = vec3(ubo.view_matrix * vec4(ubo.light_positon, 1.0));
    lightColor = ubo.light_color;

    textureIndex = pcs.texture_index;

    gl_Position = ubo.projection_matrix * ubo.view_matrix * pcs.model_matrix * vec4(aVertexPosition, 1.0f);
}