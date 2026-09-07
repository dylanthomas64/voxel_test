use bevy::prelude::*;

// chunk space constants
pub const CHUNK_SIZE: usize = 64;

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


// noise

use noise::{NoiseFn, Perlin, Fbm, MultiFractal, Seedable};

#[derive(Resource)]
pub struct TerrainNoise {
    pub perlin: Perlin,
    pub fbm: Fbm<Perlin>,
}

impl TerrainNoise {
    pub fn new(seed: u32) -> Self {
        Self {
            perlin: Perlin::new(seed),
            fbm: Fbm::<Perlin>::new(seed)
                .set_frequency(0.1)
        }
    }
}

pub fn setup_terrain_noise(mut commands: Commands) {
    commands.insert_resource(TerrainNoise::new(0));
}

// helper function to determine height of terrain
pub fn height_at(noise: &TerrainNoise, x: f32, z: f32) -> f32 {

    let amplitude = 10.0;
    let val = noise.fbm.get([x as f64, z as f64]) as f32;
    SEA_LEVEL + val * amplitude
}



pub fn generate_terrain(chunk_position: IVec3, seed: u32, terrain_noise: &TerrainNoise) -> Chunk {

    // create a cube of air voxels of volume CHUNK_SIZE**3
    let mut voxels = vec![Voxel::Air; CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE];
    const DIRT_DEPTH: i32 = 3;

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

//help functino to convert voxel type to a colour
fn palette_color(voxel: Voxel) -> [f32; 4] {
    match voxel {
        Voxel::Air => [0.0, 0.0, 0.0, 0.0], // never actually reached — Air voxels get `continue`d before this is called
        Voxel::Solid(BlockType::Grass) => [0.42, 0.62, 0.26, 1.0],
        Voxel::Solid(BlockType::Dirt) => [0.40, 0.29, 0.18, 1.0],
        Voxel::Solid(BlockType::Stone) => [0.55, 0.55, 0.55, 1.0],
        Voxel::Solid(BlockType::Water) => [0.1, 0.1, 0.988, 0.3],
        // ...one arm per block type
    }
}

// convert chunk to a single mesh (greedily)
pub fn build_chunk_mesh(chunk: &Chunk) -> Mesh {
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
                    let neighbour = chunk.get(x as i32 + dir.x, y as i32 + dir.y, z as i32 + dir.z);
                    if neighbour != Voxel::Air {
                        continue;
                    }
                    let base = positions.len() as u32;
                    let colour = palette_color(voxel);

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

// spawn the chunk into the world
pub fn spawn_chunk(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    terrain_noise: Res<TerrainNoise>,
) {
    let seed: u32 = 0;
    let render_distance = 0;

    let material = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 1.0, 1.0),
        ..default()
    });

    for cx in -render_distance..=render_distance {
        for cz in -render_distance..=render_distance {
            let chunk_position = IVec3::new(cx, 0, cz);
            let chunk = generate_terrain(chunk_position, 0, &terrain_noise);
            let mesh = build_chunk_mesh(&chunk);
            let world_offset = (chunk_position * CHUNK_SIZE as i32).as_vec3();
            commands.spawn((
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(material.clone()),
                Transform::from_translation(world_offset),
                chunk,
            ));
        }
    }
}



// **************      claude voronoi test ***************************** ///

/// Deterministic per-cell jitter in 3D, same hashing trick as before.
fn hash3(x: i32, y: i32, z: i32, seed: u32) -> Vec3 {
    let mut n = (x.wrapping_mul(374761393))
        .wrapping_add(y.wrapping_mul(668265263))
        .wrapping_add(z.wrapping_mul(2147483647))
        .wrapping_add(seed as i32) as u32;
    n = (n ^ (n >> 13)).wrapping_mul(1274126177);
    let fx = ((n & 0xffff) as f32 / 65535.0) - 0.5;
    n = n.wrapping_mul(2246822519);
    let fy = (((n >> 16) & 0xffff) as f32 / 65535.0) - 0.5;
    n = n.wrapping_mul(3266489917);
    let fz = ((n & 0xffff) as f32 / 65535.0) - 0.5;
    Vec3::new(fx, fy, fz)
}

/// Returns (distance to nearest point, distance to second-nearest point).
/// Searches the 3x3x3 neighbourhood of cells around the sample position.
pub fn voronoi3d(pos: Vec3, cell_size: f32, seed: u32) -> (f32, f32) {
    let cell = (pos / cell_size).floor().as_ivec3();

    let mut nearest = f32::MAX;
    let mut second = f32::MAX;

    for dz in -1..=1 {
        for dy in -1..=1 {
            for dx in -1..=1 {
                let ix = cell.x + dx;
                let iy = cell.y + dy;
                let iz = cell.z + dz;

                let jitter = hash3(ix, iy, iz, seed);
                let point = Vec3::new(ix as f32, iy as f32, iz as f32) * cell_size
                    + (jitter + 0.5) * cell_size;

                let d = pos.distance(point);
                if d < nearest {
                    second = nearest;
                    nearest = d;
                } else if d < second {
                    second = d;
                }
            }
        }
    }

    (nearest, second)
}


/// Layered sine waves at different frequencies/amplitudes — each octave adds
/// finer detail at lower strength, so you get big rolling shapes plus small bumps
/// instead of one uniform ripple.
fn fbm_sine(x: f32, z: f32, octaves: u32) -> f32 {
    let mut total = 0.0;
    let mut amplitude = 6.0;
    let mut frequency = 0.05;

    for _ in 0..octaves {
        total += amplitude * (x * frequency).sin() * (z * frequency).cos();
        amplitude *= 0.5;
        frequency *= 2.0;
    }
    total
}

/// World-space terrain surface height. Sea level is an absolute world constant,
/// not chunk-relative (see earlier discussion) — this function doesn't know or
/// care which chunk is asking.
fn base_height(x: f32, z: f32) -> f32 {
    let sea_level = 0.0;
    sea_level + fbm_sine(x, z, 4)
}

pub fn density_at(pos: Vec3, cell_size: f32, seed: u32) -> f32 {
    // Hard ceiling: absolutely nothing survives above this, no matter what
    // the noise says. Stops runaway floating debris outright.
    let sky_ceiling = 48.0;
    if pos.y > sky_ceiling {
        return -1.0;
    }

    // --- Base terrain field ---
    // Positive below the rolling-hill surface, negative above it.
    // This alone would give you the sine-hills-only terrain from earlier.
    let terrain_density = base_height(pos.x, pos.z) - pos.y;

    // --- Height bias, layered on top of the terrain field itself ---
    // Steeper punishment above ground than below, so noise can't easily
    // fight its way into producing solid stuff mid-air, while still leaving
    // caves free to exist underground.
    let dy = base_height(pos.x, pos.z) - pos.y;
    let bias = if dy < 0.0 { dy * 0.4 } else { dy * 0.05 };
    let terrain_density = terrain_density + bias;

    // --- Cave carving field (Voronoi) ---
    // f1 = distance to nearest feature point. Near a point (small f1) = "inside
    // a tunnel," should be air. Far from any point (large f1) = solid rock.
    let (f1, _f2) = voronoi3d(pos, cell_size, seed);
    let cave_threshold = 3.0; // bigger = fatter tunnels
    let cave_carve = f1 - cave_threshold;

    // Only let caves carve well below the surface — stops weird pockmarks
    // right at ground level / cave mouths punching straight through hillsides.
    let cave_margin = 4.0;
    if terrain_density > cave_margin {
        // Below ground and deep enough: solid only if terrain says solid
        // AND we're not inside a carved cavity (the AND/min logic).
        terrain_density.min(cave_carve)
    } else {
        // Near the surface: skip cave carving entirely, just use terrain shape.
        terrain_density
    }
}
