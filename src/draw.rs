use super::region::*;

pub struct Draw {
    screen: Region,
}

impl Draw {
    pub fn new(width: usize, height: usize) -> Draw {
        Draw {
            screen: Region::new(width, height),
        }
    }

    pub fn height(&self) -> usize {
        self.screen.height()
    }

    pub fn width(&self) -> usize {
        self.screen.width()
    }

    pub fn apply(&mut self, r: &Region) -> String {
        let height = self.screen.height();
        let width = self.screen.width();

        let mut out = String::new();

        let refresh = false;
        let mut contig = false;
        let mut redo = false;
        let def = Cell::default();

        let mut last_format = None;
        let mut last_row = None;
        let mut last_col = None;

        for y in 0..height {
            let mut x = 0;

            while x < width {
                let oc = self.screen.cell_mut(x, y).unwrap();
                let nc = if let Some(nc) = r.cell(x, y) {
                    nc
                } else {
                    &def
                };

                if !redo && oc == nc && !refresh {
                    contig = false;
                    x += oc.width();
                    continue;
                }
                redo = false;

                if !contig {
                    /*
                     * We did not write to the previous character in this row.
                     * Move the cursor into place.
                     */
                    if Some(y) == last_row {
                        if let Some(col) = last_col {
                            let skip = x - col - 1;

                            if skip == 1 {
                                /*
                                 * We would be skipping just one character.
                                 * It is generally more efficient to backtrack
                                 * and just emit that character.
                                 */
                                redo = true;
                                contig = true;
                                x -= 1;
                                continue;
                            }

                            /*
                             * It's just a jump to the right.
                             */
                            out += &format!("\x1b[{}C", skip);
                        } else {
                            /*
                             * Use an absolute column address.
                             */
                            out += &format!("\x1b[{}G", x + 1);
                        }
                    } else {
                        /*
                         * Move directly to a specific cell.
                         */
                        out += &format!("\x1b[{};{}f", y + 1, x + 1);
                    }
                }

                if last_format.is_none() {
                    last_format = Some(Format::default());
                    out += &format!("\x1b[0m");
                }

                if last_format.as_ref() != Some(nc.format()) {
                    let mut attrs = vec![0];
                    let f = nc.format();

                    if f.bold {
                        attrs.push(1);
                    }

                    if f.reverse {
                        attrs.push(7);
                    }

                    match f.fg {
                        Colour::Default => (),
                        Colour::C16(c) => attrs.push(c),
                        Colour::C256(c) => {
                            attrs.push(38);
                            attrs.push(5);
                            attrs.push(c);
                        }
                    }

                    match f.bg {
                        Colour::Default => (),
                        Colour::C16(c) => attrs.push(c),
                        Colour::C256(c) => {
                            attrs.push(48);
                            attrs.push(5);
                            attrs.push(c);
                        }
                    }

                    let s = attrs
                        .iter()
                        .map(|n| n.to_string())
                        .collect::<Vec<_>>()
                        .join(";");
                    out += &format!("\x1b[{}m", s);

                    last_format = Some(*f);
                }

                out.push(nc.contents());
                x += nc.width();

                contig = true;
                last_row = Some(y);
                last_col = Some(x - 1);

                /*
                 * Update our record of what has been drawn to the screen.
                 */
                oc.set_from(&nc);
            }
        }

        out
    }
}
