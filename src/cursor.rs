use crop::Rope;
use crossterm::terminal;

use crate::window::{Pane, PaneContainer};

pub enum CursorMove {
    Amount(usize),
    Where(usize),
    ToEnd,
    ToStart,
    Nothing,
    ToBottom,
}

pub enum Direction {
    Up,
    Down,
    Left,
    Right,
    LineStart,
    LineEnd,
    FileTop,
    FileBottom,
    PageUp,
    PageDown,
}

#[derive(Debug, Clone, Copy)]
pub struct Cursor {
    x: usize,
    y: usize,
    draw_x: usize,
    draw_y: usize,
    went_down: bool,
    went_right: bool,
    rows: usize,
    cols: usize,
    pub row_offset: usize,
    pub col_offset: usize,
    pub number_line_size: usize,
    pub ignore_offset: bool,
}

impl Cursor {
    pub fn new(win_size: (usize, usize)) -> Cursor {
        Cursor {
            x: 0,
            y: 0,
            draw_x: 0,
            draw_y: 0,
            went_down: false,
            went_right: false,
            rows: win_size.1,
            cols: win_size.0,
            row_offset: 0,
            col_offset: 0,
            number_line_size: 0,
            ignore_offset: false,
        }
    }

    pub fn set_draw_cursor(&mut self, x: usize, y: usize) {
        self.draw_x = x;
        self.draw_y = y;
    }

    pub fn get_draw_cursor(&self) -> (usize, usize) {
        (self.draw_x, self.draw_y)
    }

    pub fn resize(&mut self, win_size: (usize, usize)) {
            

        
        //let new_x = (self.x * win_size.0) as f64 / self.cols as f64;
        //let new_y = (self.y * win_size.1) as f64 / self.rows as f64;

        self.rows = win_size.1;
        self.cols = win_size.0;


        //self.x = new_x as usize;
        //self.y = new_y as usize;
    }

    pub fn set_size(&mut self, win_size: (usize, usize)) {
        self.rows = win_size.1;
        self.cols = win_size.0;
    }

    pub fn get_cursor(&self) -> (usize, usize) {
        (self.x, self.y)
    }

    pub fn get_real_cursor(&self) -> (usize, usize) {
        let x = if self.x < self.col_offset {
            self.x + self.col_offset
        }
        else {
            self.x - self.col_offset
        };

        let y = if self.y < self.row_offset {
            self.y + self.row_offset
        }
        else {
            self.y - self.row_offset
        };
        (x, y)
    }

    pub fn scroll(&mut self, pane: &PaneContainer) {
        let (pane_x, pane_y) = pane.get_size();

        if self.x >= pane_x && self.went_right {
            self.col_offset = self.x - pane_x + 1;
        }
        else if self.x < self.col_offset && !self.went_right {
            self.x = self.col_offset.saturating_sub(1);
            self.col_offset = self.col_offset.saturating_sub(1);
        }

        if self.y >= pane_y && self.went_down && self.y != 0 {
            self.row_offset = self.y - pane_y + 1;
        }
        else if self.y >= pane_y && self.went_down && self.y != 0 {
            self.row_offset = pane_y - self.y;
        }
        else if self.y < self.row_offset && !self.went_down {
            self.y = self.row_offset.saturating_sub(1);
            self.row_offset = self.row_offset.saturating_sub(1);
        }

    }

    pub fn set_cursor(&mut self, x: CursorMove, y: CursorMove, pane: &dyn Pane, (x_offset, y_offset): (usize, usize)) {
        let number_of_lines = pane.get_line_count();

        let number_of_cols = if let Some(cols) = pane.get_row_len(self.y) {
            cols.saturating_sub(x_offset)
        }
        else {
            0
        };
        

        match x {
            CursorMove::Amount(n) => {
                self.x = n % (number_of_cols + 1);
                self.draw_x = self.x;
            },
            CursorMove::Where(n) => {
                self.x = n;
                self.draw_x = self.x;
            },
            CursorMove::ToEnd | CursorMove::ToBottom => {
                self.x = self.cols.min(number_of_cols);
                self.went_right = true;
                self.draw_x = self.x;
            },
            CursorMove::ToStart => {
                self.x = 0;
                self.went_right = false;
                self.draw_x = self.x;
            },
            CursorMove::Nothing => {},
        }
        match y {
            CursorMove::Amount(n) => {
                self.y = n % (number_of_lines + 1);
                self.draw_y = self.y;
            },
            CursorMove::Where(n) => {
                self.y = n;
                self.draw_y = self.y;
            },
            CursorMove::ToEnd => {
                self.y = self.rows.min(number_of_lines);
                self.went_down = true;
                self.draw_y = self.y;
            },
            CursorMove::ToStart => {
                self.y = 0;
                self.went_down = false;
                self.draw_y = self.y;
            },
            CursorMove::ToBottom => {
                self.y = terminal::size().unwrap().1 as usize;
                self.draw_y = self.y;
            },
            CursorMove::Nothing => {},
        }
    }

    pub fn move_cursor(&mut self, direction: Direction, n: usize, pane: &dyn Pane) {
        let number_of_lines = pane.get_line_count();

        let number_of_cols = if let Some(cols) = pane.get_row_len(self.y) {
            cols
        }
        else {
            0
        };

        match direction {
            Direction::Up => {
                self.y = self.y.saturating_sub(n);
                self.went_down = false;
                self.draw_y = self.y;
            },
            Direction::Down => {
                if self.y < number_of_lines {
                    let new_y = (self.y + n) % (number_of_lines);
                    if new_y < self.y {
                        self.y = number_of_lines.saturating_sub(1);
                    }
                    else {
                        self.y = new_y;
                    }
                }
                self.went_down = true;
                self.draw_y = self.y;
            },
            Direction::Left => {
                self.x = self.x.saturating_sub(n);
                self.went_right = false;
                self.draw_x = self.x;
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
                self.went_right = true;

                self.draw_x = self.x;
            },
            Direction::LineStart => {
                self.x = 0;
                self.went_right = false;
                self.draw_x = self.x;
            },
            Direction::LineEnd => {
                self.x = number_of_cols;
                self.went_right = true;
                self.draw_x = self.x;
            },
            Direction::FileTop => {
                self.y = 0;
                self.row_offset = 0;
                self.went_down = false;
                self.draw_y = self.y;
            },
            Direction::FileBottom => {
                self.y = number_of_lines - 1;
                self.row_offset = number_of_lines.saturating_sub(self.rows + 1);
                self.went_down = true;
                self.draw_y = self.y;
            },
            Direction::PageUp => {
                self.y = self.y.saturating_sub(self.rows * n);
                self.row_offset = self.row_offset.saturating_sub(self.rows * n);
                self.went_down = false;
                self.draw_y = self.y;
            },
            Direction::PageDown => {
                let new_y = (self.y + (self.rows * n)) % number_of_lines;
                if new_y < self.y {
                    self.y = number_of_lines.saturating_sub(1);
                }
                else {
                    self.y = new_y;
                }
                self.row_offset = self.row_offset.saturating_add(self.rows * n);
                self.went_down = true;
                self.draw_y = self.y;
            },
        }
        

    }

}
