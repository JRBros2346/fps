use std::{
    collections::HashSet,
    io::{self, BufWriter, Stdout, Write as _},
    time::Duration,
};

use crossterm::{
    cursor::{Hide, RestorePosition, SavePosition, Show},
    event::{self, Event, KeyCode},
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};

include!("sixel.rs");
include!("died.rs");
include!("flag.rs");

pub struct Vizir {
    out: BufWriter<Stdout>,
    buf: Vec<u8>,
    map: [u128; 2],
    win: &'static [[u32; 2]],
    wall: [u128; 16],
}

impl Vizir {
    pub fn new(map: [u128; 2], win: &'static [[u32; 2]], wall: [u128; 16]) -> io::Result<Self> {
        let mut out = BufWriter::new(io::stdout());
        terminal::enable_raw_mode()?;
        crossterm::execute!(out, EnterAlternateScreen, Hide, SavePosition)?;
        Ok(Self {
            out,
            buf: vec![],
            map,
            win,
            wall,
        })
    }
    fn resize(&mut self, size: usize) {
        if size > self.buf.len() {
            self.buf.resize(size, 16);
        }
    }

    fn first_person(&mut self, cols: u16, rows: u16, player: [f32; 3]) -> io::Result<()> {
        // Calculate the render resolution (scaled up for sixel output)
        let w = cols as usize * 6;
        let h = rows as usize * 12;
        self.resize(w * h);
        // Fill the buffer with background color (16)
        self.buf.fill(16);

        // Raycasting parameters
        let fov: f32 = std::f32::consts::FRAC_PI_3; // 60 degrees
        let num_rays = w;
        let map_size = 16;

        let px = player[0];
        let py = player[1];
        let pa = player[2];

        // Use the WALL variable (self.wall) as a 16x16 wall texture, each texel is 8 bits in a u128 row
        let wall_tex = |x: usize, y: usize| -> u8 {
            // Each row is a u128, each texel is 8 bits, MSB first
            let row = self.wall[y & 15];
            let shift = 8 * (15 - (x & 15));
            ((row >> shift) & 0xFF) as u8
        };

        for col in 0..num_rays {
            // Calculate the angle for this ray
            let ray_angle = pa - fov / 2.0 + fov * (col as f32) / (num_rays as f32);
            let ray_dir = [ray_angle.cos(), ray_angle.sin()];

            // DDA variables
            let mut map_x = px.floor() as isize;
            let mut map_y = py.floor() as isize;
            let delta_dist_x = if ray_dir[0] == 0.0 {
                1e30
            } else {
                (1.0 / ray_dir[0]).abs()
            };
            let delta_dist_y = if ray_dir[1] == 0.0 {
                1e30
            } else {
                (1.0 / ray_dir[1]).abs()
            };
            let step_x: isize;
            let step_y: isize;
            let mut side_dist_x: f32;
            let mut side_dist_y: f32;

            if ray_dir[0] < 0.0 {
                step_x = -1;
                side_dist_x = (px - map_x as f32) * delta_dist_x;
            } else {
                step_x = 1;
                side_dist_x = (map_x as f32 + 1.0 - px) * delta_dist_x;
            }
            if ray_dir[1] < 0.0 {
                step_y = -1;
                side_dist_y = (py - map_y as f32) * delta_dist_y;
            } else {
                step_y = 1;
                side_dist_y = (map_y as f32 + 1.0 - py) * delta_dist_y;
            }

            let mut hit = false;
            let mut side = 0; // 0: x, 1: y
            let mut dist = 0.0;
            let mut wall_x = 0.0;
            for _ in 0..64 {
                if side_dist_x < side_dist_y {
                    side_dist_x += delta_dist_x;
                    map_x += step_x;
                    side = 0;
                } else {
                    side_dist_y += delta_dist_y;
                    map_y += step_y;
                    side = 1;
                }
                if map_x < 0
                    || map_x >= map_size as isize
                    || map_y < 0
                    || map_y >= map_size as isize
                {
                    break;
                }
                let bit = map_y * 16 + map_x;
                let wall = if map_y < 8 {
                    ((self.map[0] >> (127 - bit)) & 1) != 0
                } else {
                    ((self.map[1] >> (255 - bit)) & 1) != 0
                };
                if wall {
                    hit = true;
                    if side == 0 {
                        dist = (map_x as f32 - px + (1.0 - step_x as f32) / 2.0) / ray_dir[0];
                        wall_x = py + dist * ray_dir[1];
                    } else {
                        dist = (map_y as f32 - py + (1.0 - step_y as f32) / 2.0) / ray_dir[1];
                        wall_x = px + dist * ray_dir[0];
                    }
                    wall_x -= wall_x.floor();
                    break;
                }
            }

            // Calculate wall height (simple perspective)
            let wall_height = if hit {
                let corrected_dist = dist * (pa - ray_angle).cos(); // Remove fish-eye
                let height = (h as f32 / corrected_dist).min(h as f32);
                height as usize
            } else {
                0
            };

            // Draw vertical slice with texture
            let col_x = col;
            let start = h / 2 - wall_height / 2;
            let end = h / 2 + wall_height / 2;
            for y in 0..h {
                let idx = y * w + col_x;
                if wall_height > 0 && y >= start && y < end && hit {
                    // Texture mapping
                    let mut tex_x = (wall_x * 16.0) as usize & 15;
                    // Flip texture for certain sides to avoid mirroring
                    if (side == 0 && ray_dir[0] > 0.0) || (side == 1 && ray_dir[1] < 0.0) {
                        tex_x = 15 - tex_x;
                    }
                    let tex_y = (((y - start) as f32 / wall_height as f32) * 16.0) as usize & 15;
                    let color = wall_tex(tex_x, tex_y);
                    self.buf[idx] = color;
                } else if y >= end {
                    self.buf[idx] = 231; // floor color
                } else if y < start {
                    self.buf[idx] = 16; // ceiling color
                }
            }
        }

        self.emit_sixel(w, h)
    }

    fn victory(&mut self, cols: u16, rows: u16) -> io::Result<()> {
        crossterm::execute!(self.out, Clear(ClearType::All))?;
        write!(self.out, "\x1B[32m")?;
        let flag = FLAG.map(|b| b ^ 0x42);
        let flag = String::from_utf8_lossy(&flag);
        writeln!(
            self.out,
            "\x1B[{};{}H{flag}",
            rows / 2,
            (cols as usize - flag.len()) / 2 + 1
        )?;
        self.out.flush()
    }
    fn death_screen(&mut self, cols: u16, rows: u16) -> io::Result<()> {
        crossterm::execute!(self.out, Clear(ClearType::All))?;
        write!(self.out, "\x1B[31m")?;
        for (i, line) in DIED.iter().enumerate() {
            let row = (rows - DIED.len() as u16) / 2 + 1 + i as u16;
            writeln!(
                self.out,
                "\x1B[{};{}H{}",
                row,
                (cols as usize - DIED_WIDTH) / 2 + 1,
                line
            )?;
        }
        self.out.flush()
    }
    pub fn render_frame(&mut self, player: [f32; 3]) -> io::Result<()> {
        let (cols, rows) = terminal::size()?;
        if self.suffocate(player) {
            self.death_screen(cols, rows)?;
            return Ok(());
        }
        if self.win(player) {
            self.victory(cols, rows)?;
            return Ok(());
        }
        self.first_person(cols, rows, player)
    }
    fn emit_sixel(&mut self, w: usize, h: usize) -> io::Result<()> {
        write!(self.out, "{HEADER}")?;
        let mut colors = HashSet::<u8>::new();
        for band in (0..h).step_by(6) {
            colors.extend(self.buf[band * w..(band + 6) * w].iter().cloned());
            for color in colors.drain() {
                write!(self.out, "$#{color}")?;
                let mut prev = None::<char>;
                let mut count = 0;
                for x in 0..w {
                    let mut mask = 0u8;
                    for bit in 0..6 {
                        let y = band + bit;
                        if self.buf[y * w + x] == color {
                            mask |= 1 << bit;
                        }
                    }
                    if prev == Some((mask + 0x3F) as char) {
                        count += 1;
                        continue;
                    } else if let Some(p) = prev {
                        write!(self.out, "!{count}{p}")?;
                    }
                    prev = Some((mask + 0x3F) as char);
                    count = 1;
                }
                if let Some(p) = prev {
                    write!(self.out, "!{count}{p}")?;
                }
            }
            write!(self.out, "-")?; // move to next sixel row
        }

        write!(self.out, "{FOOTER}")?; // exit sixel mode
        crossterm::execute!(self.out, RestorePosition)?;
        self.out.flush()
    }

    pub fn handle_input(&mut self, player: &mut [f32; 3]) -> bool {
        if event::poll(Duration::from_millis(0)).unwrap() {
            if let Event::Key(k) = event::read().unwrap() {
                match k.code {
                    KeyCode::Up if !self.suffocate(*player) => {
                        let next_player = [
                            player[0] + player[2].cos() * 0.5,
                            player[1] + player[2].sin() * 0.5,
                            player[2],
                        ];

                        if !self.suffocate(next_player) {
                            *player = next_player;
                        }
                    }
                    KeyCode::Down if !self.suffocate(*player) => {
                        let next_player = [
                            player[0] - player[2].cos() * 0.5,
                            player[1] - player[2].sin() * 0.5,
                            player[2],
                        ];

                        if !self.suffocate(next_player) {
                            *player = next_player;
                        }
                    }
                    KeyCode::Left => player[2] -= 0.1,
                    KeyCode::Right => player[2] += 0.1,
                    KeyCode::Esc => return false,
                    _ => {}
                }
            }
        }
        true
    }
    fn suffocate(&self, player: [f32; 3]) -> bool {
        let tx = player[0] as isize;
        let ty = player[1] as isize;
        if !(0..16).contains(&tx) || !(0..16).contains(&ty) {
            true
        } else {
            let bit = ty * 16 + tx;
            if ty < 8 {
                ((self.map[0] >> (127 - bit)) & 1) != 0
            } else {
                ((self.map[1] >> (255 - bit)) & 1) != 0
            }
        }
    }
    fn win(&self, player: [f32; 3]) -> bool {
        let tx = player[0] as u32;
        let ty = player[1] as u32;
        self.win.contains(&[tx, ty])
    }
}

impl Drop for Vizir {
    fn drop(&mut self) {
        let _ = crossterm::execute!(self.out, Show, LeaveAlternateScreen);
        let _ = terminal::disable_raw_mode();
        let _ = self.out.flush();
    }
}
