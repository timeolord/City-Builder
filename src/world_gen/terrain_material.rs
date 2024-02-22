use bevy::{
    pbr::MaterialExtension,
    prelude::*,
    reflect::TypePath,
    render::render_resource::{AsBindGroup, ShaderRef},
};

#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub struct TerrainMaterial {
    /* #[uniform(100)]
    pub size: [u32; 2], */
    #[texture(100)]
    #[sampler(101)]
    pub heightmap: Handle<Image>,
}

impl MaterialExtension for TerrainMaterial {
    fn vertex_shader() -> ShaderRef {
        "shaders/terrain_material.wgsl".into()
    }
    /* fn fragment_shader() -> ShaderRef {
        "shaders/terrain_material.wgsl".into()
    } */
}
