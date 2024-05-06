#![no_std]

use spirv_std::{
    glam,
    glam::{vec3, vec4, Vec4Swizzles},
    image::SampledImage,
    num_traits::Pow,
    spirv, Image, RuntimeArray,
};

#[repr(C)]
#[derive(Copy, Clone)]
pub struct Material {
    color_texture: usize,
    base_color_factor: glam::Vec4,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct MeshPushConstants {
    model: glam::Mat4,

    material: Material,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct SceneConstants {
    view: glam::Mat4,
    proj: glam::Mat4,
    sunlight_direction: glam::Vec4,
    sunlight_color: glam::Vec4,
    ambient_color: glam::Vec4,
}

#[spirv(vertex)]
pub fn main_vs(
    #[spirv(position)] out_pos: &mut glam::Vec4,
    out_normal: &mut glam::Vec3,
    out_uv: &mut glam::Vec2,
    out_color: &mut glam::Vec4,
    out_frag_pos: &mut glam::Vec3,

    in_pos: glam::Vec3,
    in_normal: glam::Vec3,
    in_uv: glam::Vec2,
    in_color: glam::Vec4,

    #[spirv(push_constant)] mesh_input: &MeshPushConstants,

    #[spirv(uniform, descriptor_set = 0, binding = 0)] scene_data: &SceneConstants,
) {
    let view_proj = scene_data.proj* scene_data.view;

    *out_frag_pos = (mesh_input.model * in_pos.extend(0.0)).xyz();
    *out_pos = view_proj * mesh_input.model * in_pos.extend(1.0);
    *out_normal = (mesh_input.model * in_normal.extend(0.0)).xyz();
    *out_uv = in_uv;
    *out_color = in_color;
}

#[spirv(fragment)]
pub fn main_fs(
    #[spirv(front_facing)] front_facing: bool,
    output: &mut glam::Vec4,

    in_normal: glam::Vec3,
    in_uv: glam::Vec2,
    in_color: glam::Vec4,

    in_frag_pos: glam::Vec3,

    #[spirv(push_constant)] mesh_input: &MeshPushConstants,

    #[spirv(uniform, descriptor_set = 0, binding = 0)] scene_data: &SceneConstants,
    #[spirv(descriptor_set = 0, binding = 1)] textures: &RuntimeArray<
        SampledImage<Image!(2D, type=f32, sampled)>,
    >,
) {
    let color = mesh_input.material.base_color_factor
        * in_color
        * unsafe { textures.index(mesh_input.material.color_texture) }.sample(in_uv);

    let normal = if front_facing {
        in_normal.normalize()
    } else {
        -in_normal.normalize()
    };

    let sunlight_dir = scene_data.sunlight_direction.xyz().normalize();

    let diffuse = normal.dot(sunlight_dir).max(0.1) * scene_data.sunlight_color;
    let ambient = scene_data.ambient_color;

    let view_pos = (scene_data.view * vec4(0.0, 0.0, 0.0, 1.0)).xyz();

    let view_dir = (view_pos - in_frag_pos).normalize();

    let reflect_dir = -sunlight_dir - 2.0 * normal.dot(-sunlight_dir) * normal;

    let specular: glam::Vec4 =
        view_dir.dot(reflect_dir).max(0.0).pow(32) * 0.5f32 * scene_data.sunlight_color;

    *output = color * (diffuse + ambient + specular);
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct CubemapPushConstants {
    view_proj: glam::Mat4,
    model: glam::Mat4,
}

#[spirv(vertex)]
pub fn cubemap_vs(
    #[spirv(position)] out_pos: &mut glam::Vec4,
    out_uv: &mut glam::Vec3,

    #[spirv(vertex_index)] id: u32,

    #[spirv(push_constant)] cubemap_input: &CubemapPushConstants,
) {
    const VERTS: [glam::Vec3; 8] = [
        // front
        vec3(-1.0, -1.0, 1.0),
        vec3(1.0, -1.0, 1.0),
        vec3(1.0, 1.0, 1.0),
        vec3(-1.0, 1.0, 1.0),
        // back
        vec3(-1.0, -1.0, -1.0),
        vec3(1.0, -1.0, -1.0),
        vec3(1.0, 1.0, -1.0),
        vec3(-1.0, 1.0, -1.0),
    ];

    const INDECIES: [usize; 36] = [		// front
        0, 1, 2,
        2, 3, 0,
        // right
        1, 5, 6,
        6, 2, 1,
        // back
        7, 6, 5,
        5, 4, 7,
        // left
        4, 0, 3,
        3, 7, 4,
        // bottom
        4, 5, 1,
        1, 0, 4,
        // top
        3, 2, 6,
        6, 7, 3];

    let pos = VERTS[INDECIES[id as usize]];
    *out_uv = pos;

    let mut render_mat = (cubemap_input.view_proj *cubemap_input.model);

    render_mat.w_axis = vec4(0.0,0.0,0.0,1.0);
    *out_pos = render_mat * pos.extend(1.0);
}

#[spirv(fragment)]
pub fn cubemap_fs(
    output: &mut glam::Vec4,

    in_uv: glam::Vec3,

    #[spirv(descriptor_set = 0, binding = 0)] cubemap: &SampledImage<
        Image!(cube, type=f32, sampled),
    >,
) {
    *output = cubemap.sample(in_uv);
}

#[spirv(vertex)]
pub fn ui_vs(
    #[spirv(position)] out_pos: &mut glam::Vec4,
    out_uv: &mut glam::Vec2,
    out_color: &mut glam::Vec4,

    in_pos: glam::Vec2,
    in_color: glam::Vec4,
    in_uv: glam::Vec2,
) {
    *out_pos = in_pos.extend(1.0).extend(1.0);
    *out_color = in_color;
    *out_uv = in_uv;
}

#[spirv(fragment)]
pub fn ui_fs(
    output: &mut glam::Vec4,
    in_uv: glam::Vec2,
    in_color: glam::Vec4,

    #[spirv(push_constant)] texture_id: &u32,

    #[spirv(descriptor_set = 0, binding = 0)] textures: &RuntimeArray<
        SampledImage<Image!(2D, type=f32, sampled)>,
    >,
) {
    if in_uv == glam::vec2(0.0, 0.0) {
        *output = in_color;
    } else {
        *output = in_color * unsafe { textures.index(*texture_id as usize) }.sample(in_uv);
    }
}
