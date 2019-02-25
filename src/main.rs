#[macro_use]
extern crate ispc;
extern crate embree_rs;
extern crate cgmath;
extern crate tobj;
extern crate docopt;
#[macro_use]
extern crate serde_derive;
extern crate rayon;

mod tile;

use std::path::Path;
use std::mem;
use std::ptr;

use cgmath::{Vector3, Vector4, InnerSpace};
use embree_rs::{Device, Geometry, IntersectContext, RayN, RayHitN, Scene,
          TriangleMesh, CommittedScene};
use docopt::Docopt;
use rayon::prelude::*;

use tile::Tile;

ispc_module!(crescent);

static USAGE: &'static str = "
Usage:
    crescent <objfile> [OPTIONS]
    crescent (-h | --help)


Options:
  -o <path>          Specify the output file or directory to save the image or frames.
                     Supported formats are PNG, JPG and PPM.
  --eye=<x,y,z>      Specify the eye position for the camera.
  --at=<x,y,z>       Specify the position to point the camera at.
  --up=<x,y,z>       Specify the camera up vector.
  -h, --help         Show this message.
";

static WIDTH: usize = 512;
static HEIGHT: usize = 512;

#[derive(Deserialize)]
struct Args {
    arg_objfile: String,
    flag_eye: Option<String>,
    flag_at: Option<String>,
    flag_up: Option<String>,
    flag_o: Option<String>,
}


fn parse_vec_arg(s: &str) -> Vec<f32> {
    s.split(",").map(|x| x.parse::<f32>().unwrap()).collect()
}

fn main() {
    let args: Args = Docopt::new(USAGE).and_then(|d| d.deserialize()).unwrap_or_else(|e| e.exit());

    let device = Device::new();

    let (models, _) = tobj::load_obj(&Path::new(&args.arg_objfile[..])).unwrap();

    let mut tri_geoms = Vec::new();

    for m in models.iter() {
        let mesh = &m.mesh;
        println!("Mesh has {} triangles and {} verts",
                 mesh.indices.len() / 3, mesh.positions.len() / 3);

        let mut tris = TriangleMesh::unanimated(&device,
                                                mesh.indices.len() / 3,
                                                mesh.positions.len() / 3);
        {
            let mut verts = tris.vertex_buffer.map();
            let mut tris = tris.index_buffer.map();
            for i in 0..mesh.positions.len() / 3 { 
                verts[i] = Vector4::new(mesh.positions[i * 3],
                                        mesh.positions[i * 3 + 1],
                                        mesh.positions[i * 3 + 2],
                                        0.0);
            }

            for i in 0..mesh.indices.len() / 3 { 
                tris[i] = Vector3::new(mesh.indices[i * 3],
                                       mesh.indices[i * 3 + 1],
                                       mesh.indices[i * 3 + 2]);
            }
        }
        let mut tri_geom = Geometry::Triangle(tris);
        tri_geom.commit();
        tri_geoms.push(tri_geom);
    }

    let mut scene = Scene::new(&device);
    let mut mesh_ids = Vec::with_capacity(models.len());
    for g in tri_geoms.drain(0..) {
        let id = scene.attach_geometry(g);
        mesh_ids.push(id);
    }
    let rtscene = scene.commit();

    // Make the image tiles to distribute rendering work
    // TODO: Non-multiple of tile size images
    let tile_size = (32, 32);
    let mut tiles = Vec::new();
    for j in 0..HEIGHT / tile_size.1 {
        for i in 0..HEIGHT / tile_size.0 {
            tiles.push(Tile::new(tile_size, (i * tile_size.0, j * tile_size.1)));
        }
    }

    // Render the tiles
    tiles.par_iter_mut().for_each(|mut tile| render_tile(&mut tile, &rtscene, &models, &mesh_ids));

    // Now write the tiles into the final framebuffer to save out
    let mut final_image = vec![0; WIDTH * HEIGHT * 3];
    for t in tiles.iter() {
        for j in 0..t.dims.1 {
            let y = j + t.pos.1;
            for i in 0..t.dims.0 {
                let x = i + t.pos.0;
                final_image[(x + y * WIDTH) * 3] = t.srgb[(i + j * t.dims.0) * 3];
                final_image[(x + y * WIDTH) * 3 + 1] = t.srgb[(i + j * t.dims.0) * 3 + 1];
                final_image[(x + y * WIDTH) * 3 + 2] = t.srgb[(i + j * t.dims.0) * 3 + 2];
            }
        }
    }

    let out_path =
        if let Some(path) = args.flag_o {
            path
        } else {
            String::from("crescent.png")
        };
    match image::save_buffer(&out_path[..], &final_image[..], WIDTH as u32, HEIGHT as u32,
                             image::RGB(8)) {
        Ok(_) => println!("Rendered image saved to {}", out_path),
        Err(e) => panic!("Error saving image: {}", e),
    };
}

fn render_tile(tile: &mut Tile, rtscene: &CommittedScene,
               models: &Vec<tobj::Model>, mesh_ids: &Vec<u32>) {
    let mut intersection_ctx = IntersectContext::coherent();

    // Generate a stream of rays for the entire tile
    let mut rays = RayN::new(tile.dims.0 * tile.dims.1);
    unsafe {
        // TODO: Camera parameters
        let mut sys_rays = rays.as_raynp();
        crescent::generate_primary_rays(mem::transmute(&mut sys_rays),
                                        tile.pos.0 as u32, tile.pos.1 as u32,
                                        WIDTH as u32, HEIGHT as u32,
                                        tile.dims.0 as u32, tile.dims.1 as u32);
    }

    let mut ray_hit = RayHitN::new(rays);
    rtscene.intersect_stream_soa(&mut intersection_ctx, &mut ray_hit);

    unsafe {
        let mesh = &models[0].mesh;
        let normals = if mesh.normals.is_empty() { ptr::null() } else { mesh.normals.as_ptr() };
        crescent::shade_ray_stream(mem::transmute(&mut ray_hit.as_rayhitnp()),
                                    tile.dims.0 as u32, tile.dims.1 as u32,
                                    mesh.indices.as_ptr(),
                                    mesh.positions.as_ptr(),
                                    normals,
                                    tile.img.as_mut_ptr());

        crescent::image_to_srgb(tile.img.as_ptr(), tile.srgb.as_mut_ptr(),
                                tile.dims.0 as i32, tile.dims.1 as i32);
    }
}

