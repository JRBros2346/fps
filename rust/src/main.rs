use std::io;
use std::time::{Duration, Instant};
use std::{f32, thread};

use vizir::Vizir;

include!("map.rs");
include!("wall.rs");
const FRAME: Duration = Duration::from_millis(50);

fn main() {
    struct Xeno {
        engine: Vizir,
        player: [f32; 3],
        running: bool,
    }
    impl Xeno {
        fn new() -> io::Result<Self> {
            Ok(Self {
                engine: Vizir::new(MAP, WIN, WALL)?,
                player: [1.5, 14.5, 0.0],
                running: true,
            })
        }
        fn run(&mut self) -> io::Result<()> {
            while self.running {
                let t0 = Instant::now();
                self.running = self.engine.handle_input(&mut self.player);
                self.engine.render_frame(self.player)?;
                let dt = t0.elapsed();
                if dt < FRAME {
                    thread::sleep(FRAME - dt);
                }
            }
            Ok(())
        }
    }
    Xeno::new().unwrap().run().unwrap();
}
