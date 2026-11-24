pub const DEFAULT_POOL_CAPACITY: usize = 3;

#[derive(Debug)]
pub struct InstancePool<T> {
    capacity: usize,
    items: Vec<T>,
}

impl<T> InstancePool<T> {
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            items: Vec::with_capacity(capacity),
        }
    }

    pub fn acquire(&mut self) -> Option<T> {
        self.items.pop()
    }

    pub fn release(&mut self, item: T) -> Option<T> {
        if self.items.len() >= self.capacity {
            return Some(item);
        }
        self.items.push(item);
        None
    }

    pub fn drain_all(&mut self) -> Vec<T> {
        std::mem::take(&mut self.items)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    #[must_use]
    pub const fn capacity(&self) -> usize {
        self.capacity
    }
}

impl<T> Default for InstancePool<T> {
    fn default() -> Self {
        Self::new(DEFAULT_POOL_CAPACITY)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FramePlane {
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub pixels: Vec<u8>,
}

impl FramePlane {
    pub fn prepare(&mut self, width: u32, height: u32) -> bool {
        let Some(stride) = width.checked_mul(4) else {
            return false;
        };
        let Some(byte_len) = (stride as usize).checked_mul(height as usize) else {
            return false;
        };
        if width == 0 || height == 0 || byte_len == 0 {
            self.width = 0;
            self.height = 0;
            self.stride = 0;
            return false;
        }
        self.width = width;
        self.height = height;
        self.stride = stride;
        if self.pixels.len() != byte_len {
            self.pixels.resize(byte_len, 0);
        }
        true
    }

    pub fn clear_dimensions(&mut self) {
        self.width = 0;
        self.height = 0;
        self.stride = 0;
    }

    #[must_use]
    pub fn has_valid_frame(&self) -> bool {
        let Some(expected) = (self.stride as usize).checked_mul(self.height as usize) else {
            return false;
        };
        self.width > 0 && self.height > 0 && expected > 0 && self.pixels.len() >= expected
    }
}

#[derive(Debug, Default)]
pub struct DoubleFrameBuffer {
    front: FramePlane,
    spare: Option<FramePlane>,
    generation: u64,
}

impl DoubleFrameBuffer {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn checkout_render_target(&mut self, width: u32, height: u32) -> Option<FramePlane> {
        let mut target = self.spare.take().unwrap_or_default();
        if !target.prepare(width, height) {
            self.spare = Some(target);
            return None;
        }
        Some(target)
    }

    pub fn return_unused_target(&mut self, mut target: FramePlane) {
        target.clear_dimensions();
        self.spare = Some(target);
    }

    pub fn publish_rendered_frame(&mut self, mut rendered: FramePlane) -> u64 {
        if !rendered.has_valid_frame() {
            rendered.clear_dimensions();
            self.spare = Some(rendered);
            return self.generation;
        }
        std::mem::swap(&mut self.front, &mut rendered);
        rendered.clear_dimensions();
        self.spare = Some(rendered);
        self.generation = self.generation.saturating_add(1);
        self.generation
    }

    #[must_use]
    pub fn front(&self) -> Option<&FramePlane> {
        self.front.has_valid_frame().then_some(&self.front)
    }

    #[must_use]
    pub const fn generation(&self) -> u64 {
        self.generation
    }

    pub fn reset(&mut self) {
        self.front.clear_dimensions();
        if let Some(spare) = &mut self.spare {
            spare.clear_dimensions();
        }
        self.generation = self.generation.saturating_add(1);
    }
}
