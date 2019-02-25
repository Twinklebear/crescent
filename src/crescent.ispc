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

export void generate_primary_rays(uniform RTCRayNp *uniform rays,
		const uniform uint32 tile_x, const uniform uint32 tile_y,
		const uniform uint32 fb_width, const uniform uint32 fb_height,
		const uniform uint32 tile_width, const uniform uint32 tile_height)
		/*
		const uniform float *uniform pos,
		const uniform float *uniform dir_du, const uniform float *uniform dir_dv,
		const uniform float *uniform dir_top_left)
		*/
{
	foreach (i = 0 ... tile_width, j = 0 ... tile_height) {
		const uint32 ray = j * tile_width + i;
		const float px_x = (i + tile_x + 0.5f) / fb_width;
		const float px_y = (j + tile_y + 0.5f) / fb_height;

		rays->org_x[ray] = 0.0;
		rays->org_y[ray] = 0.0;
		rays->org_z[ray] = 2.5;

		vec3f dir = make_vec3f((i + tile_x + 0.5f) / fb_width - 0.5f,
				-(j + tile_y + 0.5f) / fb_height + 0.5, -1.f);
		dir = normalize(dir);

		rays->dir_x[ray] = dir.x;
		rays->dir_y[ray] = dir.y; 
		rays->dir_z[ray] = dir.z;

		/*
		rays->org_x[ray] = pos[0];
		rays->org_y[ray] = pos[1];
		rays->org_z[ray] = pos[2];

		ray->dir_x[ray] = dir_du[0] * px_x + dir_dv[0] * px_y + dir_top_left[0];
		ray->dir_y[ray] = dir_du[1] * px_x + dir_dv[1] * px_y + dir_top_left[1];
		ray->dir_z[ray] = dir_du[2] * px_x + dir_dv[2] * px_y + dir_top_left[2];
		normalize(ray->dir_x + ray, ray->dir_y + ray, ray->dir_z + ray);
		*/
	}
}

export void shade_ray_stream(uniform RTCRayHitNp *uniform ray_hit,
		const uniform uint32 tile_width, const uniform uint32 tile_height,
		const uniform uint32 *uniform indices, const uniform float *uniform verts,
		const uniform float *uniform normals,
		uniform float *uniform tile)
{
	foreach (i = 0 ... tile_width, j = 0 ... tile_height) {
		const uint32 ray = j * tile_width + i;
		const uint32 pixel = (j * tile_width + i) * 3;

		// TODO: Here we could now flip and batch by geomID
		if (ray_hit->hit.geomID[ray] != (uint32)-1) {
			const uniform uint32 *tri = indices + ray_hit->hit.primID[ray] * 3;

			const uniform float *v0 = verts + tri[0] * 3;
			const uniform float *v1 = verts + tri[1] * 3;
			const uniform float *v2 = verts + tri[2] * 3;

			vec3f a = make_vec3f(v1[0] - v0[0], v1[1] - v0[1], v1[2] - v0[2]);
			vec3f b = make_vec3f(v2[0] - v0[0], v2[1] - v0[1], v2[2] - v0[2]);
			vec3f normal = normalize(cross(a, b));

			normal = (normal + make_vec3f(1.f, 1.f, 1.f)) * 0.5f;

			tile[pixel] = normal.x;
			tile[pixel + 1] = normal.y;
			tile[pixel + 2] = normal.z;
		}
	}
}

// Convert the linear RGB framebuffer we render to to sRGB8 for saving out to an image
export void image_to_srgb(const uniform float fb[], uniform unsigned int8 srgb[],
		const uniform int32 width, const uniform int32 height)
{
	foreach (i = 0 ... width, y = 0 ... height) {
		for (int c = 0; c < 3; ++c){
			//float val = linear_to_srgb(fb[(y * width + i) * 3 + c]);
			float val = fb[(y * width + i) * 3 + c];
			srgb[(y * width + i) * 3 + c] = clamp(val * 255.0, 0.0, 255.0);
		}
	}
}

