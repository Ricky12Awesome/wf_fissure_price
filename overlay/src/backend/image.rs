use std::path::PathBuf;

use femtovg::renderer::OpenGl;
use femtovg::{Canvas, Color};
use image::DynamicImage;
use crate::backend::OverlayBackend;
use crate::{Error, OverlayConf, OverlayRenderer, OverlayTime};

#[derive(Error, Debug)]
pub enum ImageError {
    #[error("EGL display not found")]
    EglDisplayNotFound,
    #[error("EGL config not found")]
    EglConfigNotFound,
    #[error("EGL surface failed")]
    EglSurfaceFailed,
    #[error("EGL context not found")]
    EglContextNotFound,
    #[error("EGL bind api failed")]
    EglBindApiFailed,
    #[error("Canvas failed to get image")]
    CanvasFailedGetImage,
    #[error("Image creation failed")]
    ImageCreateFailed,
    #[error("OpenGL error: {0}")]
    OpenGlError(gl::types::GLenum),
    #[error("no path to save too")]
    NoSavePath,
    #[error("failed to image: {0}")]
    FailedToSave(PathBuf),
}

#[derive(Default)]
pub struct ImageBackend;

impl OverlayBackend for ImageBackend {
    type Renderer = OpenGl;

    fn run(
        &mut self,
        conf: OverlayConf,
        mut overlay: impl OverlayRenderer<Self::Renderer>,
    ) -> Result<(), Error> {
        let Some(save_path) = &conf.save_path else {
            return Err(Error::ImageError(ImageError::NoSavePath));
        };

        unsafe {
            std::env::set_var("EGL_DRIVER", "swrast");
            std::env::set_var("LIBGL_ALWAYS_SOFTWARE", "1");
        }

        let display =
            egl::get_display(egl::EGL_DEFAULT_DISPLAY).ok_or(ImageError::EglDisplayNotFound)?;

        let mut major = 0;
        let mut minor = 0;
        egl::initialize(display, &mut major, &mut minor);

        if !egl::bind_api(egl::EGL_OPENGL_ES_API) {
            return Err(Error::ImageError(ImageError::EglBindApiFailed));
        }

        #[rustfmt::skip]
        let attribs = [
            egl::EGL_RED_SIZE, 8,
            egl::EGL_GREEN_SIZE, 8,
            egl::EGL_BLUE_SIZE, 8,
            egl::EGL_ALPHA_SIZE, 8,
            egl::EGL_SURFACE_TYPE, egl::EGL_PBUFFER_BIT,
            egl::EGL_NONE,
        ];

        let config =
            egl::choose_config(display, &attribs, 1).ok_or(ImageError::EglConfigNotFound)?;

        let context_attribs = [egl::EGL_CONTEXT_CLIENT_VERSION, 2, egl::EGL_NONE];
        let context = egl::create_context(display, config, egl::EGL_NO_CONTEXT, &context_attribs)
            .ok_or(ImageError::EglContextNotFound)?;

        #[rustfmt::skip]
        let surface_attrib = [
            egl::EGL_WIDTH, conf.width as _,
            egl::EGL_HEIGHT, conf.height as _,
            egl::EGL_NONE,
        ];

        let surface = egl::create_pbuffer_surface(display, config, &surface_attrib)
            .ok_or(ImageError::EglSurfaceFailed)?;

        egl::make_current(display, surface, surface, context);

        let renderer = unsafe {
            gl::load_with(|symbol| egl::get_proc_address(symbol) as *const _);
            OpenGl::new_from_function(|symbol| egl::get_proc_address(symbol) as *const _)?
        };

        let mut canvas = Canvas::new(renderer)?;

        let mut overlay_time = OverlayTime::new();

        overlay_time.update_delta();

        canvas.set_size(conf.width, conf.height, 1.0);

        unsafe {
            gl::PixelStorei(gl::PACK_ALIGNMENT, 1);
            gl::PixelStorei(gl::PACK_ROW_LENGTH, 0);
            gl::PixelStorei(gl::PACK_SKIP_PIXELS, 0);
            gl::PixelStorei(gl::PACK_SKIP_ROWS, 0);
        }

        overlay.setup(&mut canvas, &overlay_time)?;

        canvas.clear_rect(
            0,
            0,
            canvas.width(),
            canvas.height(),
            Color::rgba(0, 0, 0, 0),
        );

        overlay.draw(&mut canvas, &overlay_time)?;

        canvas.flush();

        egl::swap_buffers(display, surface);

        let mut pixels = vec![0; (conf.width * conf.height * 4) as usize];

        unsafe {
            gl::ReadPixels(
                0,
                0,
                conf.width as _,
                conf.height as _,
                gl::RGBA,
                gl::UNSIGNED_BYTE,
                pixels.as_mut_ptr() as *mut _,
            );

            let error = gl::GetError();

            if error != gl::NO_ERROR {
                return Err(Error::ImageError(ImageError::OpenGlError(error)));
            }
        }

        let image = image::RgbaImage::from_raw(conf.width, conf.height, pixels)
            .ok_or(ImageError::ImageCreateFailed)?;

        let image = DynamicImage::ImageRgba8(image).flipv();

        image
            .save(save_path)
            .map_err(|_| ImageError::FailedToSave(save_path.to_owned()))?;

        // drop
        drop(canvas);
        egl::make_current(
            display,
            egl::EGL_NO_SURFACE,
            egl::EGL_NO_SURFACE,
            egl::EGL_NO_CONTEXT,
        );
        egl::destroy_context(display, context);
        egl::destroy_surface(display, surface);
        egl::terminate(display);

        Ok(())
    }
}
