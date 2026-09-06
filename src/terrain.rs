use bevy::prelude::*;


pub const CHUNK_SIZE: usize = 32;
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Voxel {
    Air,
    Solid(u8),
}

#[derive(Component)]
pub struct Chunk {
    pub voxels: Vec<Voxel>, // array of voxel data
    pub position: IVec3,    // chunk location
}

impl Chunk {
    fn index(x: usize, y: usize, z: usize) -> usize {
        x + y * CHUNK_SIZE + z * CHUNK_SIZE * CHUNK_SIZE
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


// helper function to determine height of terrain for sine based chunk generation
pub fn height_at_sin(x: f32, z: f32) -> f32 {
    let amplitude: f32 = 6.0;
    let frequency: f32 = 0.4;
    let sea_level: f32 = 0.5 * CHUNK_SIZE as f32;

    sea_level + amplitude * (x * frequency).sin() * (z * frequency).cos()
}


pub fn generate_terrain(chunk_position: IVec3, _seed: u32) -> Chunk {
    // create a cube of air voxels of volume CHUNK_SIZE**3
    let mut voxels = vec![Voxel::Air; CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE];
    let position = IVec3::ZERO;


    // create terrain by adding solid blocks within a specified criteria eg. height (y)

    for local_x in 0..CHUNK_SIZE {
        for local_z in 0.. CHUNK_SIZE {
            let world_x = (position.x * CHUNK_SIZE as i32 + local_x as i32) as f32;
            let world_z = (position.z * CHUNK_SIZE as i32 + local_z as i32) as f32;

            for local_y in 0..CHUNK_SIZE {
                let world_y = (position.y * CHUNK_SIZE as i32 + local_y as i32) as f32;
                let idx = local_x + local_y * CHUNK_SIZE + local_z * CHUNK_SIZE * CHUNK_SIZE;
                voxels[idx] = if world_y < height_at_sin(world_x, world_z) {
                    Voxel::Solid(1)
                } else {
                    Voxel::Air
                };
            }
        }
    }

    Chunk {
        voxels: voxels,
        position: chunk_position,
    }
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

// convert chunk to a single mesh (greedily)
pub fn build_chunk_mesh(chunk: &Chunk) -> Mesh {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();

    for x in 0..CHUNK_SIZE {
        for y in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                if chunk.get(x as i32, y as i32, z as i32) == Voxel::Air {
                    continue;
                }
                for (dir, corners, normal) in FACES.iter() {
                    let neighbour = chunk.get(x as i32 + dir.x, y as i32 + dir.y, z as i32 + dir.z);
                    if neighbour != Voxel::Air {
                        continue;
                    }
                    let base = positions.len() as u32;
                    for corner in corners {
                        positions.push([
                            corner[0] + x as f32,
                            corner[1] + y as f32,
                            corner[2] + z as f32,
                        ]);
                        normals.push(*normal);
                        uvs.push([0.0, 0.0]); // sort later
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
    .with_inserted_indices(Indices::U32(indices))
}