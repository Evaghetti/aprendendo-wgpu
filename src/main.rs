use wgpu::{CommandEncoderDescriptor, TextureViewDescriptor};
use winit::{dpi::LogicalSize, event::WindowEvent, event_loop::EventLoop, window::WindowBuilder};

const WINDOW_WIDTH: u32 = 640;
const WINDOW_HEIGHT: u32 = 480;

#[tokio::main]
async fn main() {
    let event_loop = EventLoop::new().expect("Não foi possível criar looping de eventos");
    let window = WindowBuilder::new()
        .with_inner_size(LogicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT))
        .with_resizable(false)
        .build(&event_loop)
        .expect("Não foi possível criar janela");
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);

    // Instanciar wgpu, necessário para criar surfaces e adapters
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::VULKAN,
        ..Default::default()
    });

    // Lugar que vai ser renderizado
    let surface = instance
        .create_surface(&window)
        .expect("Não foi possível criar surface parar renderizar");

    // Interface para a placa de vídeo, usado para pedir informações sobre a placa de video
    // Esta configuração é genérica, pode falhar caso não seja possível atender ela, nesse caso é
    // possível enumerar possiveis adaptadores com enumerate_adapters
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })
        .await
        .expect("Não foi possível criar adaptador para renderizar");

    // Device é um handle para a GPU, informa as features e limitações do adapter carregado e cria
    // uma Queue de renderização também
    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                required_features: adapter.features(),
                required_limits: adapter.limits(),
                label: None,
            },
            None,
        )
        .await
        .expect("Não foi possível criar device e queue para renderizar i");

    // Configurações que serão usadas quando esta Surface criar suas SurfaceTextures
    let surface_capabilities = surface.get_capabilities(&adapter);
    surface.configure(
        &device,
        &wgpu::SurfaceConfiguration {
            // Qual vai ser o uso desta configuração, RENDER_ATTACHMENT significa que
            // esta surface pode ser usada como resultado de um render pass
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            // Como esta SurfaceTexture vai ser guardado em memória, setamos para sRGB, ou o que
            // tiver
            format: *surface_capabilities
                .formats
                .iter()
                .filter(|f| f.is_srgb())
                .next()
                .unwrap_or(&surface_capabilities.formats[0]),
            // Dimensões da textura
            width: WINDOW_WIDTH,
            height: WINDOW_HEIGHT,
            // Como sincronizar a surface com o display, Fifo basicamente é vsync
            present_mode: wgpu::PresentMode::Fifo,
            // Tutorial não soube explicar :S mantendo como tá lá
            alpha_mode: surface_capabilities.alpha_modes[0],
            // Usado quando envolver texturas
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        },
    );

    let _ = event_loop.run(|event, target| match event {
        winit::event::Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => target.exit(),
        winit::event::Event::WindowEvent {
            event: WindowEvent::RedrawRequested,
            ..
        } => {
            let output = surface
                .get_current_texture()
                .expect("Não foi possível buscar textura para renderizar imagem");

            // Preciso buscar a view dessa textura para passar para a GPU
            let view = output
                .texture
                .create_view(&TextureViewDescriptor::default());
            // Buffer de comandos que são enviados para a GPU
            let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor::default());
            // Aqui vão todos os comandos que são passados para a GPU
            let _ = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    // Qual texture que vai ser desenhada, no caso a SurfaceTexture da janela
                    view: &view,
                    // Caso tenha multisampling, aqui vai a textura que vai receber o resultado,
                    // como setado para None, usa a mesma TextureView definida em view
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });

            // Faz o submit dos render passs e apresenta
            queue.submit([encoder.finish()]);
            output.present();
        }
        _ => {}
    });
}
