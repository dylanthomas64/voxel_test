use avian3d::prelude::*;
use bevy::prelude::*;

// chunk space constants
pub const CHUNK_SIZE: usize = 32;
pub const VOXEL_SIZE: usize = 1;

// world space contstants
pub const SEA_LEVEL: f32 = 0.0;
pub const HEIGHT_LIMIT: usize = 80;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Voxel {
    Air,
    Solid(BlockType),
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BlockType {
    Grass,
    Dirt,
    Stone,
    Water,
}

#[derive(Component)]
pub struct Chunk {
    pub voxels: Vec<Voxel>, // array of voxel data
}

impl Chunk {
    // do these need to be associated functions or should they just be helpers??
    fn index(x: usize, y: usize, z: usize) -> usize {
        x + y * CHUNK_SIZE + z * CHUNK_SIZE * CHUNK_SIZE
    }

    fn index_to_coord(n: usize) -> IVec3 {
        let x = n % CHUNK_SIZE;
        let y = (n / CHUNK_SIZE) % CHUNK_SIZE;
        let z = n / (CHUNK_SIZE * CHUNK_SIZE);
        IVec3::new(x as i32, y as i32, z as i32)
    }

    pub fn get(&self, x: i32, y: i32, z: i32) -> Voxel {
        if x < 0
            || y < 0
            || z < 0
            || x >= CHUNK_SIZE as i32
            || y >= CHUNK_SIZE as i32
            || z >= CHUNK_SIZE as i32
        {
            return Voxel::Air; // temporarily just give air
        }
        self.voxels[Self::index(x as usize, y as usize, z as usize)]
    }
}

use std::collections::HashMap;

#[derive(Resource, Default)]
pub struct ChunkMap {
    pub chunks: HashMap<IVec3, Chunk>,
}

// get voxel from neighbouring chunk
pub fn get_voxel(chunk_map: &ChunkMap, chunk_pos: IVec3, local: IVec3) -> Voxel {
    let mut neighbor_offset = IVec3::ZERO;
    let mut wrapped = local;

    for axis in 0..3 {
        if wrapped[axis] < 0 {
            neighbor_offset[axis] = -1;
            wrapped[axis] += CHUNK_SIZE as i32;
        } else if wrapped[axis] >= CHUNK_SIZE as i32 {
            neighbor_offset[axis] = 1;
            wrapped[axis] -= CHUNK_SIZE as i32;
        }
    }

    let target_chunk_pos = chunk_pos + neighbor_offset;

    match chunk_map.chunks.get(&target_chunk_pos) {
        Some(chunk) => chunk.get(wrapped.x, wrapped.y, wrapped.z),
        None => Voxel::Air, // neighbour not generated (yet) — treat as open air
    }
}

// noise

use noise::{Fbm, MultiFractal, NoiseFn, Perlin};

#[derive(Resource)]
pub struct TerrainNoise {
    pub _perlin: Perlin,
    pub fbm: Fbm<Perlin>,
}

impl TerrainNoise {
    pub fn new(seed: u32) -> Self {
        Self {
            _perlin: Perlin::new(seed),
            fbm: Fbm::<Perlin>::new(seed).set_frequency(0.005),
        }
    }
}

pub fn setup_terrain_noise(mut commands: Commands) {
    commands.insert_resource(TerrainNoise::new(0));
}

// helper function to determine height of terrain
pub fn height_at(noise: &TerrainNoise, x: f32, z: f32) -> f32 {
    // fbm
    let amplitude = 2.0 * CHUNK_SIZE as f32;
    let val = noise.fbm.get([x as f64, z as f64]) as f32;
    let fbm = val * amplitude;
    let ridged = (1.0 - val.abs()) * amplitude;
    fbm.lerp(
        ridged,
        noise._perlin.get([x as f64, z as f64]).clamp(0.0, 1.0) as f32,
    )
}
// make gizmo actually make a plane mesh as it refreshes every fram otherwise......
// ridged noise + fmb SEA_LEVEL + ((1.0 - val.abs()) * amplitude)
// then hyrdaulic erosion

pub fn generate_terrain(chunk_position: IVec3, terrain_noise: &TerrainNoise) -> Chunk {
    // create a cube of air voxels of volume CHUNK_SIZE**3
    let mut voxels = vec![Voxel::Air; CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE];

    // create terrain by adding solid blocks within a specified criteria eg. height (y)

    for local_x in 0..CHUNK_SIZE {
        for local_z in 0..CHUNK_SIZE {
            let world_x = (chunk_position.x * CHUNK_SIZE as i32 + local_x as i32) as f32;
            let world_z = (chunk_position.z * CHUNK_SIZE as i32 + local_z as i32) as f32;

            for local_y in 0..CHUNK_SIZE {
                let world_y = (chunk_position.y * CHUNK_SIZE as i32 + local_y as i32) as f32;
                // calculate the surface of the terrain (height)
                let height = height_at(terrain_noise, world_x, world_z);
                let idx = local_x + local_y * CHUNK_SIZE + local_z * CHUNK_SIZE * CHUNK_SIZE;
                voxels[idx] = if world_y < height {
                    Voxel::Solid(BlockType::Stone)
                } else {
                    Voxel::Air
                };
            }
        }
    }

    Chunk { voxels }
}

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, Mesh, PrimitiveTopology};

// face direction (same as normal but used for face culling), local space face coords, normals (ready for wgsl as f32)
const FACES: [(IVec3, [[f32; 3]; 4], [f32; 3]); 6] = [
    (
        IVec3::new(1, 0, 0),
        [[1., 0., 0.], [1., 1., 0.], [1., 1., 1.], [1., 0., 1.]],
        [1., 0., 0.],
    ), // +X
    (
        IVec3::new(-1, 0, 0),
        [[0., 0., 1.], [0., 1., 1.], [0., 1., 0.], [0., 0., 0.]],
        [-1., 0., 0.],
    ), // -X
    (
        IVec3::new(0, 1, 0),
        [[0., 1., 0.], [0., 1., 1.], [1., 1., 1.], [1., 1., 0.]],
        [0., 1., 0.],
    ), // +Y
    (
        IVec3::new(0, -1, 0),
        [[0., 0., 1.], [0., 0., 0.], [1., 0., 0.], [1., 0., 1.]],
        [0., -1., 0.],
    ), // -Y
    (
        IVec3::new(0, 0, 1),
        [[1., 0., 1.], [1., 1., 1.], [0., 1., 1.], [0., 0., 1.]],
        [0., 0., 1.],
    ), // +Z
    (
        IVec3::new(0, 0, -1),
        [[0., 0., 0.], [0., 1., 0.], [1., 1., 0.], [1., 0., 0.]],
        [0., 0., -1.],
    ), // -Z
];

//help functino to convert voxel type to a colour
fn palette_color(voxel: Voxel) -> [f32; 4] {
    match voxel {
        Voxel::Air => [0.0, 0.0, 0.0, 0.0], // never actually reached
        Voxel::Solid(BlockType::Grass) => [0.42, 0.62, 0.26, 1.0],
        Voxel::Solid(BlockType::Dirt) => [0.40, 0.29, 0.18, 1.0],
        Voxel::Solid(BlockType::Stone) => [0.55, 0.55, 0.55, 1.0],
        Voxel::Solid(BlockType::Water) => [0.1, 0.1, 0.988, 0.3],
    }
}

use bevy::color::{Hsla, Srgba};
use rand::RngExt;

fn jitter_colour(rgba: [f32; 4], rng: &mut impl RngExt) -> [f32; 4] {
    let mut hsla: Hsla = Srgba::from_f32_array(rgba).into();

    let lightness_shift = rng.random_range(-0.2f32..=0.2);
    if lightness_shift >= 0.0 {
        hsla = hsla.lighter(lightness_shift)
    } else {
        hsla = hsla.darker(-lightness_shift)
    };

    hsla.saturation = (hsla.saturation + rng.random_range(-0.2f32..=0.2)).clamp(0.0, 1.0);
    let out: Srgba = hsla.into();
    out.to_f32_array()
}

// convert chunk to a single mesh (greedily)
pub fn build_chunk_mesh(chunk_map: &ChunkMap, chunk_pos: IVec3) -> Mesh {
    let mut rng = rand::rng();

    let chunk = chunk_map
        .chunks
        .get(&chunk_pos)
        .expect("chunk must exist to be meshed");

    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut colours = Vec::new();
    let mut indices = Vec::new();

    for x in 0..CHUNK_SIZE {
        for y in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                let voxel = chunk.get(x as i32, y as i32, z as i32);
                if voxel == Voxel::Air {
                    continue;
                }
                for (dir, corners, normal) in FACES.iter() {
                    // local chunk coordinate NOTE: this can be outside of the CHUNK_SIZE range
                    let local_coord =
                        IVec3::new(x as i32 + dir.x, y as i32 + dir.y, z as i32 + dir.z);
                    let neighbour = get_voxel(chunk_map, chunk_pos, local_coord);
                    if neighbour != Voxel::Air {
                        continue;
                    }
                    let base = positions.len() as u32;
                    let mut colour = palette_color(voxel);
                    colour = jitter_colour(colour, &mut rng);

                    for corner in corners {
                        positions.push([
                            corner[0] + x as f32,
                            corner[1] + y as f32,
                            corner[2] + z as f32,
                        ]);
                        normals.push(*normal);
                        uvs.push([0.0, 0.0]); // sort later
                        colours.push(colour)
                    }
                    indices.extend_from_slice(&[
                        base,
                        base + 1,
                        base + 2,
                        base,
                        base + 2,
                        base + 3,
                    ]);
                }
            }
        }
    }
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    // colour will be blended with base_colour by standard material
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colours)
    .with_inserted_indices(Indices::U32(indices))
}

// local colliders
fn build_chunk_collider(chunk_map: &ChunkMap, chunk_pos: IVec3) -> Collider {
    let chunk = chunk_map
        .chunks
        .get(&chunk_pos)
        .expect("chunk must exist to be have a collider");
    let solid_voxels: Vec<IVec3> = chunk
        .voxels
        .iter()
        .enumerate()
        .filter(|(_, voxel)| **voxel != Voxel::Air)
        .map(|(n, _)| Chunk::index_to_coord(n))
        .collect();
    Collider::voxels(Vec3::splat(1.0), &solid_voxels)
}

// spawn the chunk into the world
pub fn spawn_chunk(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    terrain_noise: Res<TerrainNoise>,
    mut chunk_map: ResMut<ChunkMap>,
) {
    let xz_render_distance = 3;
    let y_render_distance = 3;

    // because building the chunks meshes relies on the neighbour chunks existing we must generate all the neighbouring chunks first

    for cx in -xz_render_distance..=xz_render_distance {
        for cz in -xz_render_distance..=xz_render_distance {
            for cy in -y_render_distance..=y_render_distance {
                let chunk_position = IVec3::new(cx, cy, cz);
                let chunk = generate_terrain(chunk_position, &terrain_noise);
                chunk_map.chunks.insert(chunk_position, chunk);
            }
        }
    }

    let material = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 1.0, 1.0),
        ..default()
    });

    for cx in -xz_render_distance..=xz_render_distance {
        for cz in -xz_render_distance..=xz_render_distance {
            for cy in -y_render_distance..=y_render_distance {
                let chunk_position = IVec3::new(cx, cy, cz);
                let mesh = build_chunk_mesh(&chunk_map, chunk_position);

                // some chunks may be full air or stone and have no mesh, it'll work but bevy will complain so:
                if mesh.count_vertices() == 0 {
                    continue;
                }
                let world_offset = (chunk_position * CHUNK_SIZE as i32).as_vec3();

                commands.spawn((
                    Mesh3d(meshes.add(mesh)),
                    MeshMaterial3d(material.clone()),
                    Transform::from_translation(world_offset),
                    RigidBody::Static,
                    build_chunk_collider(&chunk_map, chunk_position), // builds chunks locally but the transform componet above will move it so OK
                ));
            }
        }
    }
}
