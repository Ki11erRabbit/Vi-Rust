use crop::Rope;

use crate::window::Pane;

pub enum CursorMove {
    Amount(usize),
    ToEnd,
    ToStart,
    Nothing,
}

pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Debug)]
pub struct Cursor {
    x: usize,
    y: usize,
    rows: usize,
    cols: usize,
    pub row_offset: usize,
    pub col_offset: usize,
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

    pub fn scroll(&mut self, pane: &Pane) {
        let (pane_x, pane_y) = pane.get_size();

        if self.x >= pane_x {
            self.col_offset = self.x - pane_x + 1;
        }
        else {
            self.col_offset = 0;
        }

        if self.y >= pane_y {
            self.row_offset = self.y - pane_y + 1;
        }
        else {
            self.row_offset = 0;
        }

    }

    pub fn set_cursor(&mut self, x: CursorMove, y: CursorMove, rows: &Rope, (x_offset, y_offset): (usize, usize)) {
        
        let mut number_of_lines = rows.line_len();
        if let Some(newline) = rows.chars().last() {
            if newline != '\n' {
                number_of_lines += 1;
            }
        }
        number_of_lines = number_of_lines.saturating_sub(y_offset);
        let number_of_cols = if let Some(row) = rows.lines().nth(self.y.saturating_sub(y_offset)) {
            row.chars().count().saturating_sub(x_offset)
        }
        else {
            0
        };

        match x {
            CursorMove::Amount(n) => {
                self.x = n % (number_of_cols + 1);
            },
            CursorMove::ToEnd => {
                self.x = self.cols.min(number_of_cols);
            },
            CursorMove::ToStart => {
                self.x = 0;
            },
            CursorMove::Nothing => {},
        }
        match y {
            CursorMove::Amount(n) => {
                self.y = n % (number_of_lines + 1)
            },
            CursorMove::ToEnd => {
                self.y = self.rows.min(number_of_lines);
            },
            CursorMove::ToStart => {
                self.y = 0;
            },
            CursorMove::Nothing => {},
        }
    }

    pub fn move_cursor(&mut self, direction: Direction, n: usize, rows: &Rope) {
        let mut number_of_lines = rows.line_len();
        if let Some(newline) = rows.chars().last() {
            if newline != '\n' {
                number_of_lines += 1;
            }
        }
        let number_of_cols = if let Some(row) = rows.lines().nth(self.y) {
            row.chars().count()
        }
        else {
            0
        };

        match direction {
            Direction::Up => {
                self.y = self.y.saturating_sub(n);
            },
            Direction::Down => {
                if self.y < number_of_lines {
                    let new_y = (self.y + n) % (number_of_lines + 1);
                    if new_y < self.y {
                        self.y = number_of_lines;
                    }
                    else {
                        self.y = new_y;
                    }
                }
            },
            Direction::Left => {
                self.x = self.x.saturating_sub(n);
            },
            Direction::Right => {
                if self.x < number_of_cols {
                    let new_x = (self.x + n) % (number_of_cols + 1);

                    if new_x < self.x {
                        self.x = number_of_cols;
                    }
                    else {
                        self.x = new_x;
                    }

                }
                else {
                    self.x = number_of_cols;
                }
            },
        }
        

    }

}
