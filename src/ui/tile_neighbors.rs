// For each tile, store a bitmask of 8 bits for neighbor occupancy (N, NE, E, SE, S, SW, W, NW)
// 1 = filled, 0 = empty/air
#[derive(Clone, Default)]
pub struct TileNeighbors(pub u8);

impl TileNeighbors {
    pub fn is_internal(&self) -> bool {
        self.0 == 0b1111_1111
    }
    pub fn from_grid<T, F: Fn(T) -> bool>(grid: &Vec<Vec<T>>, x: usize, y: usize, is_filled: F) -> Self
    where T: Copy {
        let mut mask = 0u8;
        let dirs = [(-1,0),( -1,1), (0,1), (1,1), (1,0), (1,-1), (0,-1), (-1,-1)];
        let h = grid.len() as isize;
        let w = if h > 0 { grid[0].len() as isize } else { 0 };
        for (i, (dy, dx)) in dirs.iter().enumerate() {
            let ny = y as isize + dy;
            let nx = x as isize + dx;
            if ny >= 0 && ny < h {
                let row = &grid[ny as usize];
                if nx >= 0 && (nx as usize) < row.len() {
                    if is_filled(row[nx as usize]) {
                        mask |= 1 << i;
                    }
                }
            }
        }
        TileNeighbors(mask)
    }
}
