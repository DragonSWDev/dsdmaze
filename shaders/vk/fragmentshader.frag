#version 450 core

layout (location = 0) in vec2 textureCoords;
layout (location = 1) in vec3 normalVector;
layout (location = 2) in vec3 fragmentPosition;
layout (location = 3) in vec3 lightPosition;
layout (location = 4) in vec3 lightColor;
layout (location = 5) in flat int textureIndex;

layout (binding = 1) uniform sampler samp;
layout (binding = 2) uniform texture2D textures[4];

layout(location = 0) out vec4 FragColor;

void main()
{
    //Phong shading

    //Ambient
    float ambientStrenght = 0.1;
    vec3 ambient = ambientStrenght * lightColor;

    //Diffuse
    vec3 normal = normalize(normalVector);
    vec3 lightDirection = normalize(lightPosition - fragmentPosition);
    float diff = max(dot(normal, lightDirection), 0.0);
    vec3 diffuse = diff * lightColor;

    //Specular
    float specularStrength = 0.5;
    vec3 viewDirection = normalize(-fragmentPosition);
    vec3 reflectDirection = reflect(-lightDirection, normalVector);
    float spec = pow(max(dot(viewDirection, reflectDirection), 0.0), 64);
    vec3 specular = specularStrength * spec * lightColor;

    //Point light attentuation
    float distance = length(lightPosition - fragmentPosition);
    float attenuation = 1.0 / (1.0 + 0.8 * distance + 2.4 * (distance * distance));

    ambient *= attenuation;
    diffuse *= attenuation;
    specular *= attenuation;

    vec3 fragmentResult = (ambient + diffuse + specular) * texture(sampler2D(textures[textureIndex], samp), textureCoords).rgb;

    FragColor = vec4(fragmentResult, 1.0);
}