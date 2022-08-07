#[derive(Clone, Copy, PartialEq)]
pub struct Point {
    x: usize,
    y: usize,
}

pub struct Region {
    width: usize,
    height: usize,
    rows: Vec<Vec<Cell>>,
    cursor: Option<Point>,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Colour {
    Default,
    C16(u8),
    C256(u8),
    RGB(u8, u8, u8),
}

impl Colour {
    pub fn as_rgb(&self) -> (u8, u8, u8) {
        match self {
            Colour::RGB(r, g, b) => (*r, *g, *b),
            _ => panic!("wrong type"),
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct Format {
    pub bold: bool,
    pub reverse: bool,
    pub fg: Colour,
    pub bg: Colour,
}

impl Default for Format {
    fn default() -> Self {
        Format {
            bold: false,
            reverse: false,
            fg: Colour::Default,
            bg: Colour::Default,
        }
    }
}

#[derive(PartialEq)]
pub struct Cell {
    contents: char,
    width: usize,
    format: Format,
}

impl Default for Cell {
    fn default() -> Self {
        Cell { contents: ' ', width: 1, format: Format::default() }
    }
}

impl Cell {
    pub fn chr(&mut self, ch: char) -> usize {
        /*
         * XXX deal with character width.
         */
        self.contents = ch;
        self.width = 1;
        self.width
    }

    pub fn contents(&self) -> char {
        self.contents
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn clear(&mut self) {
        self.set_from(&Cell::default());
    }

    pub fn reset(&mut self) {
        self.format = Format::default();
    }

    pub fn bold(&mut self) {
        self.format.bold = true;
    }

    pub fn reverse(&mut self) {
        self.format.reverse = true;
    }

    pub fn format(&self) -> &Format {
        &self.format
    }

    pub fn set_format(&mut self, f: &Format) {
        self.format = *f;
    }

    pub fn set_from(&mut self, other: &Cell) {
        self.contents = other.contents;
        self.width = other.width;
        self.format = other.format;
    }
}

impl Region {
    pub fn new(width: usize, height: usize) -> Region {
        let mut rows = Vec::with_capacity(height);

        while rows.len() < height {
            let mut col = Vec::with_capacity(width);
            while col.len() < width {
                col.push(Cell::default());
            }

            rows.push(col);
        }

        Region { width, height, rows, cursor: None }
    }

    pub fn cell(&self, x: usize, y: usize) -> Option<&Cell> {
        if x >= self.width || y >= self.height {
            None
        } else {
            Some(&self.rows[y][x])
        }
    }

    pub fn cell_mut(&mut self, x: usize, y: usize) -> Option<&mut Cell> {
        if x >= self.width || y >= self.height {
            None
        } else {
            Some(&mut self.rows[y][x])
        }
    }

    pub fn chr(&mut self, x: usize, y: usize, ch: char) -> usize {
        if let Some(c) = self.cell_mut(x, y) {
            c.reset();
            c.chr(ch)
        } else {
            /*
             * Don't write off the edge of the screen.
             */
            0
        }
    }

    pub fn chrf(&mut self, x: usize, y: usize, ch: char, f: &Format) -> usize {
        if let Some(c) = self.cell_mut(x, y) {
            c.set_format(f);
            c.chr(ch)
        } else {
            /*
             * Don't write off the edge of the screen.
             */
            0
        }
    }

    pub fn str(&mut self, mut x: usize, y: usize, s: &str) -> usize {
        let ox = x;

        /* XXX graphemes? */
        for ch in s.chars() {
            x += self.chr(x, y, ch);
        }

        x - ox
    }

    pub fn strf(
        &mut self,
        mut x: usize,
        y: usize,
        s: &str,
        f: &Format,
    ) -> usize {
        let ox = x;

        /* XXX graphemes? */
        for ch in s.chars() {
            x += self.chrf(x, y, ch, f);
        }

        x - ox
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn clear(&mut self) {
        for y in 0..self.height {
            for x in 0..self.width {
                self.rows[y][x].clear();
            }
        }
    }

    pub fn cursor(&self) -> Option<Point> {
        self.cursor
    }

    pub fn set_cursor(&mut self, curs: Option<Point>) {
        self.cursor = curs;
    }
}
