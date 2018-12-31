#[macro_use]
extern crate ispc;
extern crate sol;
extern crate cgmath;
extern crate tobj;

use std::path::Path;

use cgmath::{Vector3, Vector4, InnerSpace};
use sol::{Device, Geometry, IntersectContext, RayN, RayHitN, Scene, TriangleMesh};

ispc_module!(crescent);

fn main() {
    let width = 512;
    let height = 512;
    let mut framebuffer = vec![0.0; width * height * 3];
    let mut srgb_img_buf = vec![0u8; width * height * 3];
    let device = Device::new();

    let args: Vec<_> = std::env::args().collect();
    let (models, _) = tobj::load_obj(&Path::new(&args[1])).unwrap();
    let mesh = &models[0].mesh;

    println!("Mesh has {} triangles and {} verts",
             mesh.indices.len() / 3, mesh.positions.len() / 3);

    // Make a triangle
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

    let mut scene = Scene::new(&device);
    scene.attach_geometry(tri_geom);
    let rtscene = scene.commit();

    let mut intersection_ctx = IntersectContext::coherent();

    // Render the scene
    for j in 0..height {
        let y = -(j as f32 + 0.5) / height as f32 + 0.5;

        // Try out streams of scanlines across x
        let mut rays = RayN::new(width);
        for (i, mut ray) in rays.iter_mut().enumerate() {
            let x = (i as f32 + 0.5) / width as f32 - 0.5;
            let dir_len = f32::sqrt(x * x + y * y + 1.0);
            ray.set_origin(Vector3::new(0.0, 0.0, 3.5));
            ray.set_dir(Vector3::new(x / dir_len, y / dir_len, -1.0 / dir_len));
        }

        let mut ray_hit = RayHitN::new(rays);
        rtscene.intersect_stream_soa(&mut intersection_ctx, &mut ray_hit);
        for (i, hit) in ray_hit.hit.iter().enumerate().filter(|(_i, h)| h.hit()) {
            let uv = hit.uv();
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

                framebuffer[(i + j * width) * 3] = n.x;
                framebuffer[(i + j * width) * 3 + 1] = n.y;
                framebuffer[(i + j * width) * 3 + 2] = n.z;
            } else {
                framebuffer[(i + j * width) * 3] = uv.0;
                framebuffer[(i + j * width) * 3 + 1] = uv.1;
                framebuffer[(i + j * width) * 3 + 2] = 0.0;
            }
        }
    }

    unsafe {
        crescent::framebuffer_to_srgb(framebuffer.as_ptr(), srgb_img_buf.as_mut_ptr(),
                                      width as i32, height as i32);
    }

    match image::save_buffer("out.png", &srgb_img_buf[..], width as u32, height as u32,
                             image::RGB(8)) {
        Ok(_) => println!("Rendered image saved to out.png"),
        Err(e) => panic!("Error saving image: {}", e),
    };
}

