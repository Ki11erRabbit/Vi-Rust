use crop::Rope;



pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}


pub struct Cursor {
    x: usize,
    y: usize,
    rows: usize,
    cols: usize,
    row_offset: usize,
    col_offset: usize,
}

impl Cursor {
    pub fn new(win_size: (usize, usize)) -> Cursor {
        Cursor {
            x: 0,
            y: 0,
            rows: win_size.1,
            cols: win_size.0,
            row_offset: 0,
            col_offset: 0,
        }
    }

    pub fn get_cursor(&self) -> (usize, usize) {
        (self.x, self.y)
    }

    pub fn get_real_cursor(&self) -> (usize, usize) {
        (self.x - self.col_offset, self.y - self.row_offset)
    }

    pub fn move_cursor(&mut self, direction: Direction, n: usize, rows: &Rope) {
        let number_of_lines = rows.line_len();

        match direction {
            Direction::Up => {
                self.y = self.y.saturating_sub(n);
            },
            Direction::Down => {
                if self.y < number_of_lines {
                    self.y = (self.y + n) % number_of_lines;
                }
            },
            Direction::Left => {
                if self.x.saturating_sub(n) != 0 {
                    self.x = self.x.saturating_sub(n);
                }
            },
            Direction::Right => {
                if self.x < self.cols {
                    self.x = (self.x + n) % self.cols;
                }
            },
        }
        

    }

}
