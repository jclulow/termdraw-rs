use rand::prelude::*;
use chrono::prelude::*;
use chrono_tz::Tz;
use std::collections::HashSet;
use std::io::Read;
use std::mem::MaybeUninit;
use std::os::unix::io::{AsRawFd, RawFd};
use std::sync::mpsc;
use std::time::{Duration, Instant};
use termdraw::Colour;
use termios::{TCIOFLUSH, TCSADRAIN, TCSANOW};

const YELLOW: Colour = Colour::RGB(0xF5, 0xCF, 0x75);
const OFF_WHITE: Colour = Colour::RGB(0xD0, 0xD0, 0xD0);
const GREEN_LIGHT: Colour = Colour::RGB(0x48, 0xD5, 0x97);
const GREEN_DARK: Colour = Colour::RGB(0x11, 0x27, 0x25);
const GREEN_DARKEST: Colour = Colour::RGB(0x0B, 0x14, 0x18);

struct WinSize {
    height: usize,
    width: usize,
}

fn emit(io: &mut dyn std::io::Write, data: &str) -> std::io::Result<()> {
    io.write_all(data.as_bytes())?;
    io.flush()?;
    Ok(())
}

fn getwinsz(fd: RawFd) -> std::io::Result<WinSize> {
    let mut winsize: MaybeUninit<libc::winsize> = MaybeUninit::uninit();
    let r = unsafe { libc::ioctl(fd, libc::TIOCGWINSZ, winsize.as_mut_ptr()) };
    if r != 0 {
        Err(std::io::Error::last_os_error())
    } else {
        let winsize = unsafe { winsize.assume_init() };
        Ok(WinSize {
            height: winsize.ws_row as usize,
            width: winsize.ws_col as usize,
        })
    }
}

pub fn main() {
    let mut rng = rand::thread_rng();
    let mut stdout = std::io::stdout();
    let sz = getwinsz(stdout.as_raw_fd()).unwrap();

    let mut draw = termdraw::Draw::new(sz.width, sz.height);
    let mut r = termdraw::Region::new(draw.width(), draw.height());

    let tz: Tz = "US/Pacific".parse().unwrap();

    let nodename =
        hostname::get().unwrap_or("?".into()).to_str().unwrap().to_string();

    let mut quit = false;
    let mut go = true;

    /*
     * Create a thread to process input from stdin.
     */
    let (tx, rx) = mpsc::sync_channel(0);
    std::thread::spawn(move || {
        let stdin = std::io::stdin();
        let mut br = std::io::BufReader::new(stdin);
        let mut buf = [0u8; 1];

        loop {
            match br.read(&mut buf) {
                Ok(0) => return,
                Ok(1) => {
                    if tx.send(buf[0]).is_err() {
                        return;
                    }
                }
                Ok(n) => panic!("{} is not the correct number of bytes", n),
                _ => return,
            };
        }
    });

    /*
     * Put the terminal in raw mode.
     */
    let orig_termios = termios::Termios::from_fd(stdout.as_raw_fd()).unwrap();
    let mut termios = orig_termios.clone();
    termios::cfmakeraw(&mut termios);
    termios::tcsetattr(stdout.as_raw_fd(), TCSANOW, &termios).unwrap();
    termios::tcflush(stdout.as_raw_fd(), TCIOFLUSH).unwrap();

    let mut fc = 0;
    let mut deadline = Instant::now();
    'outer: loop {
        if quit {
            go = false;
        }
        if !go {
            break;
        }

        fc += 10;

        r.clear();

        let hh = 3;
        let hf = 2;

        let mut f = termdraw::Format::default();
        f.bg = GREEN_DARK;
        f.fg = GREEN_LIGHT;

        let mut yf = termdraw::Format::default();
        yf.bg = GREEN_DARK;
        yf.fg = YELLOW;

        let mut ff = termdraw::Format::default();
        ff.bg = GREEN_DARKEST;
        ff.fg = OFF_WHITE;

        let oxide = include_str!("oxide.txt")
            .lines()
            .map(|s| s.chars().collect::<Vec<_>>())
            .collect::<Vec<_>>();
        let oxidew = oxide.iter().map(|l| l.len()).max().unwrap_or(0);

        for y in 0..hh {
            for x in 0..r.width() {
                r.chrf(x, y, ' ', &f);
            }
        }
        for y in hh..(r.height() - hf - 1) {
            for x in 0..r.width() {
                r.chrf(x, y, ' ', &ff);
            }
        }
        for y in (r.height() - hf - 1)..r.height() {
            for x in 0..r.width() {
                r.chrf(x, y, ' ', &f);
            }
        }

        let msgl = "OXIDE COMPUTER COMPANY";
        let msgr = "PROGRAMMING STATION";

        r.strf(3, 1, msgl, &f);
        r.strf(r.width() - 2 - msgr.len(), 1, msgr, &f);


        let ftrl = format!("STATION: {}", nodename.to_ascii_uppercase());
        r.strf(3, r.height() - 2, &ftrl, &yf);

        let now = Utc::now().with_timezone(&tz);
        let ftrr = now.format("%Y-%m-%d %H:%M:%S").to_string();
        r.strf(r.width() - 2 - ftrr.len(), r.height() - 2, &ftrr, &yf);

        let offs = r.width() - oxidew - 1;
        let hoff = r.height() - hf - oxide.len() - 2;

        for y in 0..oxide.len() {
            for x in 0..oxidew {
                if x >= oxide[y].len() {
                    continue;
                }

                if oxide[y][x] == '#' {
                    r.chrf(offs + x, hoff + y, ' ', &f);
                }
            }
        }

        let out = draw.apply(&r);
        if !emit(&mut stdout, &out).is_ok() {
            break;
        }

        /*
         * Set the target time for the next frame relative to the time for the
         * current frame.
         */
        let now = Instant::now();
        deadline = deadline.checked_add(Duration::from_millis(80)).unwrap();
        if deadline.lt(&now) {
            /*
             * The selected target time is already in the past, which implies
             * that the target frame rate is too high for this system.
             */
            deadline = now;
        }

        /*
         * Listen for input messages until it's time to draw the next frame.
         */
        loop {
            let rem = deadline.saturating_duration_since(Instant::now());
            if rem.is_zero() {
                break;
            }

            if let Ok(c) = rx.recv_timeout(rem) {
                if c == 0x03 {
                    /*
                     * ^C means exit now.
                     */
                    break 'outer;
                }
                if c == b'q' || c == b'Q' {
                    quit = true;
                    break;
                }
            }
        }
    }

    /*
     * Clean up the terminal and restore the original termios attributes:
     */
    emit(&mut stdout, &draw.cleanup()).ok();
    termios::tcsetattr(stdout.as_raw_fd(), TCSADRAIN, &orig_termios).unwrap();
}
