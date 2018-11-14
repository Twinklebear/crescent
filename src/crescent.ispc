#include "vec3f.ih"
#include "ray.ih"

float linear_to_srgb(const float f) {
	if (f <= 0.0031308){
		return 12.92 * f;
	} else {
		return 1.055 * pow(f, 1.0 / 2.4) - 0.055;
	}
}

export void render(const void *uniform scene, const uniform int32 width,
		const uniform int32 height, uniform float img[])
{
	for (uniform int y = 0; y < height; ++y) {
		foreach (i = 0 ... width) {
			// Just send rays through the pixel centers for now
			const float px = (i + 0.5 - width / 2.f) / (width / 2.f);
			const float py = -(y + 0.5 - height / 2.f) / (height / 2.f);

			RayHit1 ray_hit;
			ray_hit.ray.org = make_vec3f(0, 0, 0);
			ray_hit.ray.dir = normalize(make_vec3f(px, py, -1));
			ray_hit.ray.t_near = 0;
			ray_hit.ray.t_far = 1e20f;
			ray_hit.ray.mask = -1;
			ray_hit.ray.id = 0;
			ray_hit.ray.flags = 0;

			ray_hit.hit.primID = -1;
			ray_hit.hit.geomID = -1;
			ray_hit.hit.instID = -1;
			img[(y * width + i) * 3] = 1;
			img[(y * width + i) * 3 + 1] = 0;
			img[(y * width + i) * 3 + 2] = 0;
		}
	}
}

// Convert the linear RGB framebuffer we render to to sRGB8 for saving out to an image
export void framebuffer_to_srgb(const uniform float fb[], uniform unsigned int8 srgb[],
		const uniform int32 width, const uniform int32 height)
{
	for (uniform int y = 0; y < height; ++y) {
		foreach (i = 0 ... width){
			for (int c = 0; c < 3; ++c){
				float val = linear_to_srgb(fb[(y * width + i) * 3 + c]);
				srgb[(y * width + i) * 3 + c] = clamp(val * 255.0, 0.0, 255.0);
			}
		}
	}
}



