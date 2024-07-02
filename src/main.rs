use std::hash::{DefaultHasher, Hash, Hasher};

use bevy::{
    prelude::*,
    render::{
        render_asset::RenderAssetUsages,
        render_resource::{Extent3d, TextureDimension, TextureFormat},
    },
};
use bevy_inspector_egui::{
    bevy_egui::{EguiContexts, EguiPlugin},
    egui,
};

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Diamond-Square Implementation".to_string(),
                        resolution: (1000., 1000.).into(),
                        ..default()
                    }),
                    ..default()
                }),
        )
        .add_event::<GenTileEvent>()
        .add_plugins(EguiPlugin)
        .add_systems(Update, bevy::window::close_on_esc)
        .add_systems(Startup, setup)
        .add_systems(Update, process_gentile)
        .add_systems(Update, ui_example)
        .run();
}

#[derive(Event, Debug)]
struct GenTileEvent {
    pub position: Position,
    pub seed: isize,
    pub image_size: usize,
    pub roughness: f32,
}

#[derive(Component)]
struct Tile;

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
struct Position((i32, i32));

fn setup(mut commands: Commands, mut gentile: EventWriter<GenTileEvent>) {
    const DEFAULT_TILE_SIZE: usize = 2usize.pow(9) + 1;

    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 000.0, 1.0),
        ..Default::default()
    });

    // Setup initial tile.
    gentile.send(GenTileEvent {
        position: Position((0, 0)),
        seed: 0,
        roughness: 2.0,
        image_size: DEFAULT_TILE_SIZE,
    });
}

fn process_gentile(
    mut commands: Commands,
    mut event: EventReader<GenTileEvent>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for tile_event in event.read() {
        let (px, py) = tile_event.position.0;

        // Create the texture from dynamically generated image.
        let texture = images.add(Image::new(
            Extent3d {
                width: tile_event.image_size as u32,
                height: tile_event.image_size as u32,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            generate_map(
                tile_event.position,
                tile_event.roughness,
                tile_event.seed,
                tile_event.image_size,
            ),
            TextureFormat::Rgba8Unorm,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        ));

        // Spawn in a quad with the generated image.
        commands.spawn((
            PbrBundle {
                mesh: meshes.add(Rectangle::new(1.0, 1.0)),
                material: materials.add(StandardMaterial {
                    base_color_texture: Some(texture.clone()),
                    double_sided: true,
                    cull_mode: None,
                    unlit: true,
                    alpha_mode: AlphaMode::Blend,
                    ..Default::default()
                }),
                transform: Transform::from_xyz((px as i32) as f32, (py as i32) as f32, 0.0),
                ..Default::default()
            },
            Tile,
        ));
    }
}

fn ui_example(
    mut contexts: EguiContexts,
    mut gentile: EventWriter<GenTileEvent>,
    mut commands: Commands,
    mut sprite_query: Query<(Entity, &Tile)>,
    mut seed: Local<isize>,
    mut roughness: Local<Option<f32>>,
    mut node_size: Local<Option<usize>>,
) {
    const DEFAULT_ROUGHNESS: f32 = 2.0;
    const DEFAULT_NODE_SIZE: usize = 6;

    // Initialize default values if they are not set yet.
    if roughness.is_none() {
        *roughness = Some(DEFAULT_ROUGHNESS);
    }

    if node_size.is_none() {
        *node_size = Some(DEFAULT_NODE_SIZE);
    }

    // Settings window.
    egui::Window::new("Terrain Generation Settings").show(contexts.ctx_mut(), |ui| {
        ui.label(format!("Seed: {}", *seed));
        ui.add(egui::Slider::new(roughness.as_mut().unwrap(), 1.0..=6.0).prefix("Roughness: "));
        ui.add(egui::Slider::new(node_size.as_mut().unwrap(), 4..=10).prefix("Node Size"));

        if ui.button("Generate Terrain").clicked() {
            // Clear all tiles. Should only be one.
            for (entity, _) in sprite_query.iter_mut() {
                commands.entity(entity).despawn();
            }

            // Generate a new seed.
            *seed = rand::random();

            // Send an event to generate a new tile.
            gentile.send(GenTileEvent {
                position: Position((0, 0)),
                seed: *seed,
                roughness: roughness.unwrap(),
                image_size: 2usize.pow(node_size.unwrap() as u32) + 1,
            });
        }
    });
}

fn generate_map(position: Position, roughness: f32, seed: isize, image_size: usize) -> Vec<u8> {
    // this has to be dynamically allocated because the image is not static.
    let mut heightmap: Vec<Vec<f32>> = vec![vec![0.0; image_size]; image_size];

    let mut chunk_size = image_size - 1;
    let mut roughness = roughness;

    // Easy cordnate to hash function. Allowing for unique but persistent outputs.
    let hash = |x: i32, y: i32| {
        let mut hasher = DefaultHasher::new();
        hasher.write_isize(seed);
        hasher.write_i32(x);
        hasher.write_i32(y);
        let res = (hasher.finish() % 0xFF) as f32 / 0xFF as f32;
        res
    };

    
    // Set values for all four corners.
    let (px, py) = position.0;
    heightmap[0][0] = hash(px, py);
    heightmap[0][image_size - 1] = hash(px, py + 1);
    heightmap[image_size - 1][0] = hash(px + 1, py);
    heightmap[image_size - 1][image_size - 1] = hash(px + 1, py + 1);

    // The Diamond-Square algorithm.
    while chunk_size > 1 {
        let half = chunk_size / 2;

        //square step
        for y in (0..image_size - 1).step_by(chunk_size) {
            for x in (0..image_size - 1).step_by(chunk_size) {
                let top_left = heightmap[x][y];
                let top_right = heightmap[x + chunk_size][y];
                let bottom_left = heightmap[x][y + chunk_size];
                let bottom_right = heightmap[x + chunk_size][y + chunk_size];

                let average = (top_left + top_right + bottom_left + bottom_right) / 4.0;
                heightmap[x + half][y + half] = average;

                let random_factor = hash(x as i32, y as i32) * 2.0 - 1.0;
                let random_offset = random_factor * roughness;
                heightmap[x][y] += random_offset;
            }
        }

        // diamond step
        for y in (0..image_size).step_by(half) {
            for x in ((y + half) % chunk_size..(image_size)).step_by(chunk_size) {
                let mut neighbors = 0;
                let mut neighbor_sum = 0.0;

                if x > half {
                    neighbors += 1;
                    neighbor_sum += heightmap[x - half][y];
                }

                if y > half {
                    neighbors += 1;
                    neighbor_sum += heightmap[x][y - half];
                }

                if x + half < image_size - 1 {
                    neighbors += 1;
                    neighbor_sum += heightmap[x + half][y];
                }

                if y + half < image_size - 1 {
                    neighbors += 1;
                    neighbor_sum += heightmap[x][y + half];
                }

                heightmap[x][y] = neighbor_sum / neighbors as f32;

                let random = hash(x as i32, y as i32) * 2.0 - 1.0;
                let random = random * roughness;
                heightmap[x][y] += random;
            }
        }

        chunk_size /= 2;
        roughness /= 2.0;
    }

    // Transform the raw data into a usable format.
    heightmap
        .into_iter()
        .flatten()
        // Plug each value into logistics curve to clamp (0-1).
        .map(|f| 1.0 / (1.0 + std::f32::consts::E.powf(-f)))
        // Apply basic coloring based on value.
        .map(|f| {
            let value = (f * 0xFF as f32) as i32;
            match f {
                f if f < 0.2 => (value) << 0,
                f if f < 0.65 => (value) << 8,
                f if f < 0.9 => (value / 2 << 16) | (value / 2 << 8) | value / 2,
                _ => (value << 16) | (value << 8) | value,
            }
        })
        // Convert to a color format that Bevy can use.
        .map(|f| {
            let r = ((f >> 16) & 0xFF) as u8;
            let g = ((f >> 8) & 0xFF) as u8;
            let b = (f & 0xFF) as u8;
            [r, g, b, 0xFF]
        })
        .flatten()
        .collect()
}
