#![no_std]
use spirv_std::*;

#[repr(C)]
pub struct WorldUniform {
    view_proj: glam::Mat4,
}

#[repr(C)]
pub struct ChunkUniform {
    pos: glam::IVec3,
}

#[spirv(vertex)]
pub fn main_vs(
    #[spirv(position)] out_pos: &mut glam::Vec4,
    out_uv: &mut glam::Vec2,

    in_pos: glam::Vec3,
    in_uv: glam::Vec2,

    #[spirv(uniform, descriptor_set = 0, binding = 0)] world_uniform: &WorldUniform,

    #[spirv(uniform, descriptor_set = 1, binding = 0)] chunk_uniform: &ChunkUniform,
) {
    *out_uv = in_uv;
    *out_pos = world_uniform.view_proj * (in_pos + chunk_uniform.pos.as_vec3() * 16.0).extend(1.0);
}

#[spirv(fragment)]
pub fn main_fs(
    out: &mut glam::Vec4,

    in_uv: glam::Vec2,

    #[spirv(descriptor_set = 0, binding = 1)] texture: &Image!(2D, type=f32, sampled),
    #[spirv(descriptor_set = 0, binding = 2)] sampler: &Sampler,
) {
    *out = texture.sample(*sampler, in_uv);
}
