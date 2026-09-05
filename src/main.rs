use bevy::prelude::*;

const CHUNK_SIZE: usize = 16;

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

fn spawn_chunk(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let chunk = Chunk {
        voxels: vec![Voxel::Solid(1); CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE],
        position: IVec3::ZERO,
    };
    let mesh = build_chunk_mesh(&chunk);

    commands.spawn((
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(materials.add(Color::srgb(0.3, 0.6, 0.3))),
        Transform::default(),
        chunk,
    ));
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    spawn_camera(commands.reborrow());
    spawn_chunk(commands.reborrow(), meshes, materials);
    // spawn light as standard material requires it
    commands.spawn((
        DirectionalLight::default(),
        Transform::default().looking_to(Vec3::new(-1.0, -1.0, -0.5), Vec3::Y),
    ));
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(24.0, 24.0, 24.0).looking_at(Vec3::new(8.0, 8.0, 8.0), Vec3::Y),
    ));
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}
