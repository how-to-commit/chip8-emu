use super::core::{SCREEN_HEIGHT, SCREEN_WIDTH};

pub enum ProgramState {
    Running,
    // WaitingForInput,
    Finished,
    // Paused,
}

pub enum TimerState {
    PlaySound,
    None,
}

pub struct Screen {
    inner: [bool; SCREEN_WIDTH * SCREEN_HEIGHT],
}

impl Screen {
    pub fn new() -> Self {
        Self {
            inner: [false; SCREEN_WIDTH * SCREEN_HEIGHT],
        }
    }

    pub fn reset(&mut self) {
        self.inner.fill(false);
    }

    fn coordinate_to_index<T>(x: T, y: T) -> usize
    where
        T: Into<usize>,
    {
        // handle overflow
        let ix = x.into() % SCREEN_WIDTH;
        let iy = y.into() % SCREEN_HEIGHT;
        (SCREEN_WIDTH * ix) + iy
    }

    pub fn get_pixel<T>(&self, x: T, y: T) -> bool
    where
        T: Into<usize>,
    {
        self.inner[Screen::coordinate_to_index(x, y)]
    }

    pub fn set_pixel<T>(&mut self, x: T, y: T, val: bool) -> bool
    where
        T: Into<usize>,
    {
        let idx = Screen::coordinate_to_index(x, y);
        let res = self.inner[idx] == val;
        self.inner[idx] = val;
        res
    }

    pub fn iter_screen() {
        todo!()
    }
}
