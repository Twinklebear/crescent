#[macro_use]
extern crate ispc;
extern crate sol;
extern crate cgmath;

use cgmath::{Vector3, Vector4};
use sol::{Device, Geometry, IntersectContext, RayN, RayHitN, Scene, TriangleMesh};

ispc_module!(crescent);

fn main() {
    let width = 512;
    let height = 512;
    let mut framebuffer = vec![0.0; width * height * 3];
    let mut srgb_img_buf = vec![0u8; width * height * 3];
    // Can I do scene setup in Rust and build everything, then just
    // hand off to ISPC for rendering?
    // TODO: Yes, the handles for embree are the same across everything
    // so it's fine to setup everything in Rust and then pass
    // the scene handle over to ISPC
    let device = Device::new();

    // Make a triangle
    let mut triangle = TriangleMesh::unanimated(&device, 1, 3);
    {
        let mut verts = triangle.vertex_buffer.map();
        let mut tris = triangle.index_buffer.map();
        verts[0] = Vector4::new(-1.0, 0.0, 0.0, 0.0);
        verts[1] = Vector4::new(0.0, 1.0, 0.0, 0.0);
        verts[2] = Vector4::new(1.0, 0.0, 0.0, 0.0);

        tris[0] = Vector3::new(0, 1, 2);
    }
    let mut tri_geom = Geometry::Triangle(triangle);
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
            ray.set_origin(Vector3::new(0.0, 0.5, 2.0));
            ray.set_dir(Vector3::new(x / dir_len, y / dir_len, -1.0 / dir_len));
        }

        let mut ray_hit = RayHitN::new(rays);
        rtscene.intersect_stream_soa(&mut intersection_ctx, &mut ray_hit);
        for (i, hit) in ray_hit.hit.iter().enumerate().filter(|(_i, h)| h.hit()) {
            let uv = hit.uv();
            framebuffer[(i + j * width) * 3] = uv.0;
            framebuffer[(i + j * width) * 3 + 1] = uv.1;
            framebuffer[(i + j * width) * 3 + 2] = 0.0;
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

