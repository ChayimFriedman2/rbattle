#![allow(dead_code)]

// `error_chain!` can recurse deeply
#![recursion_limit = "1024"]

#[macro_use] extern crate error_chain;
#[macro_use] extern crate glium;
extern crate rand;

#[cfg(test)]
#[macro_use]
mod test_utils;

mod drawer;
mod errors;
mod graph;
mod map;
mod math;
mod mouse;
mod square;
mod state;
mod visible_graph;

use drawer::Drawer;
use graph::Graph;
use map::{Map, Player};
use math::{apply, compose};
use mouse::Mouse;
use square::SquareGrid;
use state::{MAX_GOOP, Occupied, State};
use visible_graph::GraphPt;

use glium::glutin::{Event, ElementState, MouseButton, VirtualKeyCode};
use glium::Surface;

use std::rc::Rc;
use std::time::{Duration, Instant};

// This only gives access within this module. Make this `pub use errors::*;`
// instead if the types must be accessible from other modules (e.g., within
// a `links` section).
use errors::*;

fn main() {
    if let Err(ref e) = run() {
        use ::std::io::Write;
        let stderr = &mut ::std::io::stderr();
        let errmsg = "Error writing to stderr";

        writeln!(stderr, "error: {}", e).expect(errmsg);

        for e in e.iter().skip(1) {
            writeln!(stderr, "caused by: {}", e).expect(errmsg);
        }

        // The backtrace is not always generated. Try to run this example
        // with `RUST_BACKTRACE=1`.
        if let Some(backtrace) = e.backtrace() {
            writeln!(stderr, "backtrace: {:?}", backtrace).expect(errmsg);
        }

        ::std::process::exit(1);
    }
}

fn run() -> Result<()> {
    use glium::DisplayBuild;

    let display = glium::glutin::WindowBuilder::new()
        .with_title("rbattle".to_string())
        .build_glium()
        .chain_err(|| "unable to open window")?;

    let graph = SquareGrid::new(15, 15);
    let sources = vec![16, 45];
    let colors = vec![(0x9f, 0x20, 0xb1), (0xb1, 0x20, 0x44),
                      (0x20, 0xb1, 0x21), (0x20, 0x67, 0xb1),
                      (0xe0, 0x6f, 0x3a)];
    let map = Rc::new(Map::new(graph, sources, colors));
    let drawer = Drawer::new(&display, &map)
        .chain_err(|| "failed to construct Drawer for map")?;

    let mut state = State::new(map.clone());

    state.nodes[45] = Some(Occupied {
        player: Player(2),
        outflows: map.graph.neighbors(45),
        goop: MAX_GOOP
    });

    state.nodes[30] = Some(Occupied {
        player: Player(2),
        outflows: vec![15],
        goop: 0
    });

    state.nodes[16] = Some(Occupied {
        player: Player(0),
        outflows: map.graph.neighbors(16),
        goop: MAX_GOOP
    });

    state.nodes[17] = Some(Occupied {
        player: Player(1),
        outflows: vec![],
        goop: 2
    });

    let mut mouse = Mouse::new(map.clone());
    let mut single_step = true;

    loop {
        let mut frame = display.draw();
        frame.clear_color(1.0, 1.0, 1.0, 1.0);
        let status = drawer.draw(&mut frame, &state, &mouse);
        frame.finish()
            .chain_err(|| "drawing finish failed")?;

        let window_to_game = status?;
        let window_to_graph = compose(map.game_to_graph, window_to_game);

        for event in display.poll_events() {
            match event {
                Event::Closed => return Ok(()),
                Event::KeyboardInput(ElementState::Pressed, _,
                                     Some(VirtualKeyCode::Space)) => {
                    single_step = true;
                    state.flow();
                    state.generate_goop();
                }
                Event::KeyboardInput(ElementState::Pressed, _,
                                     Some(VirtualKeyCode::Return)) => {
                    single_step = false;
                }
                Event::MouseMoved(x, y) => {
                    let graph_pos = apply(window_to_graph, [x as f32, y as f32]);
                    mouse.move_to(GraphPt(graph_pos));
                }
                Event::MouseInput(ElementState::Pressed, MouseButton::Left) => {
                    mouse.click();
                }
                Event::MouseInput(ElementState::Released, MouseButton::Left) => {
                    if let Some(action) = mouse.release() {
                        state.take_action(action);
                    }
                }
                _ => ()
            }
        }

        if !single_step {
            state.flow();
            state.generate_goop();
        }
    }
}
