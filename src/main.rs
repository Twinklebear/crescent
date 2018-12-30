#[macro_use]
extern crate ispc;

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
    unsafe {
        crescent::render(std::ptr::null(), width as i32, height as i32,
                         framebuffer.as_mut_ptr());
        crescent::framebuffer_to_srgb(framebuffer.as_ptr(), srgb_img_buf.as_mut_ptr(),
                                      width as i32, height as i32);
    }

    match image::save_buffer("out.png", &srgb_img_buf[..], width as u32, height as u32,
                             image::RGB(8)) {
        Ok(_) => println!("Rendered image saved to out.png"),
        Err(e) => panic!("Error saving image: {}", e),
    };
}

