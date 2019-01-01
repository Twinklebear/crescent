/// An image tile
pub struct Tile {
    pub dims: (usize, usize),
    pub pos: (usize, usize),
    pub img: Vec<f32>,
    pub srgb: Vec<u8>,
}

impl Tile {
    pub fn new(dims: (usize, usize), pos: (usize, usize)) -> Tile {
        let img = vec![0.0; dims.0 * dims.1 * 3];
        let srgb = vec![0; dims.0 * dims.1 * 3];
        Tile { dims: dims, pos: pos, img: img, srgb: srgb }
    }
}

