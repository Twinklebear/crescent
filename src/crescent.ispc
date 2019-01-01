#include "vec3f.ih"
#include "ray.ih"

float linear_to_srgb(const float f) {
	if (f <= 0.0031308){
		return 12.92 * f;
	} else {
		return 1.055 * pow(f, 1.0 / 2.4) - 0.055;
	}
}

// Convert the linear RGB framebuffer we render to to sRGB8 for saving out to an image
export void image_to_srgb(const uniform float fb[], uniform unsigned int8 srgb[],
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



