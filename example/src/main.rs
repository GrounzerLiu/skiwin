use skia_safe::Color;
use skiwin::vulkan::VulkanSkiaWindow;
use skiwin::SkiaWindow;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};
use skiwin::cpu::SoftSkiaWindow;
use skiwin::gl::GlWindow;

#[derive(Default)]
struct App {
    window: Option<Box<dyn SkiaWindow>>
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        println!("Resumed");
        if self.window.is_none() {
            let window = event_loop.create_window(Window::default_attributes()).unwrap();
            let window = GlWindow::new(window);
            self.window = Some(Box::new(window));
        }/*else {
            self.window.as_mut().unwrap().resumed();
        }*/
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                println!("The close button was pressed; stopping");
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                self.window.as_mut().unwrap().resize().unwrap();
            }
            WindowEvent::RedrawRequested => {
                let window = self.window.as_mut().unwrap();
                let window_size = window.inner_size();
                let surface = window.surface();
                let canvas = surface.canvas();
                canvas.clear(Color::BLACK);
                canvas.draw_rect(skia_safe::Rect::from_wh(100.0, 100.0), skia_safe::Paint::default().set_color(Color::BLUE));
                let mut paint = skia_safe::Paint::default();
                paint.set_color(Color::RED);
                let left = window_size.width as f32 - 100.0;
                let top = window_size.height as f32 - 100.0;
                canvas.draw_rect(skia_safe::Rect::from_xywh(left, top, 100.0, 100.0), &paint);
                window.present();
                window.request_redraw();
            }
            _ => (),
        }
    }
    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        println!("Suspended");
    }
}

fn main() {
    let event_loop = EventLoop::new().unwrap();

    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = App::default();
    event_loop.run_app(&mut app).unwrap();
}
