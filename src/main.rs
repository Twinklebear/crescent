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
    for j in 0..tile.dims.1 {
        let y = -((j + tile.pos.1) as f32 + 0.5) / HEIGHT as f32 + 0.5;

        // Try out streams of scanlines across x
        let mut rays = RayN::new(tile.dims.0);
        for (i, mut ray) in rays.iter_mut().enumerate() {
            let x = ((i + tile.pos.0) as f32 + 0.5) / WIDTH as f32 - 0.5;
            let dir_len = f32::sqrt(x * x + y * y + 1.0);
            ray.set_origin(Vector3::new(0.0, 0.0, 3.5));
            ray.set_dir(Vector3::new(x / dir_len, y / dir_len, -1.0 / dir_len));
        }

        let mut ray_hit = RayHitN::new(rays);
        rtscene.intersect_stream_soa(&mut intersection_ctx, &mut ray_hit);
        for (i, hit) in ray_hit.hit.iter().enumerate().filter(|(_i, h)| h.hit()) {
            let uv = hit.uv();
            let mesh = &models[mesh_ids[hit.geom_id() as usize] as usize].mesh;
            if !mesh.normals.is_empty() {
                let prim = hit.prim_id() as usize;
                let tri = [mesh.indices[prim * 3] as usize,
                           mesh.indices[prim * 3 + 1] as usize,
                           mesh.indices[prim * 3 + 2] as usize];

                let na = Vector3::new(mesh.normals[tri[0] * 3],
                                      mesh.normals[tri[0] * 3 + 1],
                                      mesh.normals[tri[0] * 3 + 2]);

                let nb = Vector3::new(mesh.normals[tri[1] * 3],
                                      mesh.normals[tri[1] * 3 + 1],
                                      mesh.normals[tri[1] * 3 + 2]);

                let nc = Vector3::new(mesh.normals[tri[2] * 3],
                                      mesh.normals[tri[2] * 3 + 1],
                                      mesh.normals[tri[2] * 3 + 2]);

                let w = 1.0 - uv.0 - uv.1;
                let mut n = (na * w + nb * uv.0 + nc * uv.1).normalize();
                n = (n + Vector3::new(1.0, 1.0, 1.0)) * 0.5;

                tile.img[(i + j * tile.dims.0) * 3] = n.x;
                tile.img[(i + j * tile.dims.0) * 3 + 1] = n.y;
                tile.img[(i + j * tile.dims.0) * 3 + 2] = n.z;
            } else {
                tile.img[(i + j * tile.dims.0) * 3] = uv.0;
                tile.img[(i + j * tile.dims.0) * 3 + 1] = uv.1;
                tile.img[(i + j * tile.dims.0) * 3 + 2] = 0.0;
            }
        }
    }
    unsafe {
        crescent::image_to_srgb(tile.img.as_ptr(), tile.srgb.as_mut_ptr(),
                                tile.dims.0 as i32, tile.dims.1 as i32);
    }
}

