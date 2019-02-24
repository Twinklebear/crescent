#include <embree3/rtcore_ray.isph>
#include "vec3f.ih"
#include "ray.ih"

typedef unsigned int8 uint8;
typedef unsigned int uint32;

float linear_to_srgb(const float f) {
	if (f <= 0.0031308){
		return 12.92 * f;
	} else {
		return 1.055 * pow(f, 1.0 / 2.4) - 0.055;
	}
}

void normalize(uniform float *x, uniform float *y, uniform float *z) {
	const float len = sqrt(*x * *x + *y * *y + *z * *z);
	*x /= len;
	*y /= len;
	*z /= len;
}

export void generate_primary_rays(uniform RTCRayHitNp *uniform ray_hit,
		const uniform uint32 tile_x, const uniform uint32 tile_y,
		const uniform uint32 fb_width, const uniform uint32 fb_height,
		const uniform uint32 tile_width, const uniform uint32 tile_height,
		const uniform float *uniform pos,
		const uniform float *uniform dir_du, const uniform float *uniform dir_dv,
		const uniform float *uniform dir_top_left)
{
	foreach (i = 0 ... tile_width, j = 0 ... tile_height) {
		const uint32 ray = j * tile_width + i;
		const float px_x = (i + tile_x + 0.5f) / fb_width;
		const float px_y = (j + tile_y + 0.5f) / fb_height;

		ray_hit->ray.org_x[ray] = pos[0];
		ray_hit->ray.org_y[ray] = pos[1];
		ray_hit->ray.org_z[ray] = pos[2];

		ray_hit->ray.dir_x[ray] = dir_du[0] * px_x + dir_dv[0] * px_y + dir_top_left[0];
		ray_hit->ray.dir_y[ray] = dir_du[1] * px_x + dir_dv[1] * px_y + dir_top_left[1];
		ray_hit->ray.dir_z[ray] = dir_du[2] * px_x + dir_dv[2] * px_y + dir_top_left[2];
		normalize(ray_hit->ray.dir_x + ray, ray_hit->ray.dir_y + ray,
				ray_hit->ray.dir_z + ray);
	}
}

// Convert the linear RGB framebuffer we render to to sRGB8 for saving out to an image
export void image_to_srgb(const uniform float fb[], uniform unsigned int8 srgb[],
		const uniform int32 width, const uniform int32 height)
{
	foreach (i = 0 ... width, y = 0 ... height) {
		for (int c = 0; c < 3; ++c){
			float val = linear_to_srgb(fb[(y * width + i) * 3 + c]);
			srgb[(y * width + i) * 3 + c] = clamp(val * 255.0, 0.0, 255.0);
		}
	}
}



