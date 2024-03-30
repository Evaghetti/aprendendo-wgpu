use winit::{event::WindowEvent, event_loop::EventLoop, window::WindowBuilder};

fn main() {
    let event_loop = EventLoop::new().expect("Não foi possível criar looping de eventos");
    let _window = WindowBuilder::new()
        .build(&event_loop)
        .expect("Não foi possível criar janela");

    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
    let _ = event_loop.run(|event, target| match event {
        winit::event::Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => target.exit(),
        winit::event::Event::WindowEvent {
            event: WindowEvent::RedrawRequested,
            ..
        } => println!("Renderizar"),
        _ => {}
    });
}
