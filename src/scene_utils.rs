use kurbo::{Affine, Rect, Shape, Size, Stroke};
use peniko::{BlendMode, BrushRef, Color, Fill, Font, Image};
use vello::{DrawGlyphs, Scene};

pub struct SizedScene<'a> {
    pub scene: &'a mut Scene,
    pub size: Size,
}

impl<'a> SizedScene<'a> {
    /// Creates a new scene.
    pub fn new(scene: &'a mut Scene, size: Size) -> SizedScene<'a> {
        SizedScene { scene, size }
    }

    pub fn push_layer(
        &mut self,
        blend: impl Into<BlendMode>,
        alpha: f32,
        transform: Affine,
        clip: &impl Shape,
    ) {
        self.scene.push_layer(blend, alpha, transform, clip);
    }

    pub fn pop_layer(&mut self) {
        self.scene.pop_layer();
    }

    pub fn draw_blurred_rounded_rect(
        &mut self,
        transform: Affine,
        rect: Rect,
        brush: Color,
        radius: f64,
        std_dev: f64,
    ) {
        self.scene
            .draw_blurred_rounded_rect(transform, rect, brush, radius, std_dev);
    }

    pub fn draw_blurred_rounded_rect_in(
        &mut self,
        shape: &impl Shape,
        transform: Affine,
        rect: Rect,
        brush: Color,
        radius: f64,
        std_dev: f64,
    ) {
        self.scene.draw_blurred_rounded_rect_in(
            shape, transform, rect, brush, radius, std_dev,
        );
    }

    pub fn fill<'b>(
        &mut self,
        style: Fill,
        transform: Affine,
        brush: impl Into<BrushRef<'b>>,
        brush_transform: Option<Affine>,
        shape: &impl Shape,
    ) {
        self.scene
            .fill(style, transform, brush, brush_transform, shape);
    }

    pub fn stroke<'b>(
        &mut self,
        style: &Stroke,
        transform: Affine,
        brush: impl Into<BrushRef<'b>>,
        brush_transform: Option<Affine>,
        shape: &impl Shape,
    ) {
        self.scene
            .stroke(style, transform, brush, brush_transform, shape);
    }

    pub fn draw_image(&mut self, image: &Image, transform: Affine) {
        self.scene.draw_image(image, transform);
    }

    pub fn draw_glyphs(&mut self, font: &Font) -> DrawGlyphs<'_> {
        self.scene.draw_glyphs(font)
    }

    pub fn append_scene(&mut self, other: &Scene, transform: Option<Affine>) {
        self.scene.append(other, transform);
    }

    pub fn append(&mut self, other: &SizedScene, transform: Option<Affine>) {
        self.scene.append(other.scene, transform);
    }
}
