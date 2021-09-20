use std::collections::HashSet;
use std::time::{Instant, Duration};
use std::sync::mpsc;
use std::io::Read;
use std::os::unix::io::{RawFd, AsRawFd};
use termios::{TCIOFLUSH, TCSANOW, TCSADRAIN};
use rand::prelude::*;
use std::mem::MaybeUninit;

struct Orb {
    word: String,
    x: usize,
    y: usize,
    frame: usize,
    rate: usize,
    active: bool,
    starter: bool,
    ramp: &'static [u8],
}

struct WinSize {
    height: usize,
    width: usize,
}

const MISSION: &[&str] = &[
    "kick butt",
    "have fun",
    "don't cheat",
    "love our customers",
    "change computing forever",
];

const PRINCIPLES: &[&str] = &[
    "integrity",
    "honesty",
    "decency",
];

const VALUES: &[&str] = &[
    "candor",
    "courage",
    "curiosity",
    "diversity",
    "empathy",
    "humor",
    "optimism",
    "resilience",
    "responsibility",
    "rigor",
    "teamwork",
    "thriftiness",
    "transparency",
    "urgency",
    "versatility",
];

const GREY_RAMP: &[u8] = &[ 232, 233, 234, 235, 236, 237, 238, 239, 240, 241,
242, 243, 244, 245, 246, 247, 248, 249, 250, 251, 252, 253, 254, 255, ];
const BLUE_RAMP: &[u8] = &[ 17, 18, 18, 19, 19, 20, 20, 21, 27, 32, 33,
    38, 39, 44, 45, 45, 81, 81, 51, 51, 123, 123, ];
const GREEN_RAMP: &[u8] = &[ 22, 22, 22, 28, 28, 34, 34, 40, 40, 46, 46, 46, ];

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

    print!("\x1b[H\x1b[2J\x1b[?25l");

    let msg = "press q to quit...";
    let mut orbs = vec![
        Orb {
            x: (draw.width() - msg.len()) / 2,
            y: draw.height() / 2,
            word: msg.into(),
            active: true,
            frame: 0,
            rate: 1,
            starter: true,
            ramp: BLUE_RAMP,
        }
    ];
    let mut inuse = HashSet::new();
    let mut quit = false;
    let mut go = false;

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
                Ok(1) => if tx.send(buf[0]).is_err() {
                    return;
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

    let mut deadline = Instant::now();
    'outer: loop {
        if go && !quit {
            if rng.gen_bool(0.25) {
                for _ in 0..10 {
                    /*
                     * Cheap version of not overlapping: just make sure no words
                     * are on the same row together.
                     */
                    let y = rng.gen_range(0..draw.height());
                    if orbs.iter().any(|orb| orb.y == y) {
                        continue;
                    }

                    let which = rng.gen_range(0..=100);
                    let (words, ramp, rate_range) = {
                        if which < 55 {
                            (VALUES, GREY_RAMP, 1..=3)
                        } else if which < 85 {
                            (PRINCIPLES, GREEN_RAMP, 1..=1)
                        } else {
                            (MISSION, BLUE_RAMP, 1..=1)
                        }
                    };

                    let word = words[rng.gen_range(0..words.len())].to_string();

                    if inuse.insert(word.clone()) {
                        orbs.push(Orb {
                            x: rng.gen_range(0..(draw.width() - word.len())),
                            y,
                            word,
                            frame: 0,
                            rate: rng.gen_range(rate_range),
                            active: true,
                            starter: false,
                            ramp,
                        });
                        break;
                    }
                }
            }
        } else if orbs.is_empty() {
            break;
        }

        r.clear();

        for orb in orbs.iter_mut() {
            let mut f = termdraw::Format::default();
            f.fg = termdraw::Colour::C256(if orb.frame < orb.ramp.len() {
                orb.ramp[orb.frame]
            } else if orb.frame < orb.ramp.len() * 2 {
                let idx = orb.ramp.len() - 1 - (orb.frame - orb.ramp.len());
                orb.ramp[idx]
            } else {
                orb.active = false;
                continue;
            });
            orb.frame += orb.rate;

            r.strf(orb.x, orb.y, &orb.word, &f);
        }

        while let Some(i) = orbs.iter().position(|orb| !orb.active) {
            let rem = orbs.swap_remove(i);
            if rem.starter {
                go = true;
            }
            inuse.remove(&rem.word);
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
                    /*
                     * Every orb should decay at a faster rate once the user has
                     * asked us to quit:
                     */
                    orbs.iter_mut().for_each(|orb| orb.rate = 6);
                    quit = true;
                    break;
                }
            }
        }
    }

    /*
     * Clean up the terminal and restore the original termios attributes:
     */
    emit(&mut stdout, &format!("\x1b[{};{}f\x1b[?25h", draw.height(), 1)).ok();
    termios::tcsetattr(stdout.as_raw_fd(), TCSADRAIN, &orig_termios).unwrap();
}
