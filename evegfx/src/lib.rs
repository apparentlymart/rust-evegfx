#![no_std]

pub mod display_list;
pub mod graphics_mode;
pub mod host_commands;
pub mod interface;
pub mod low_level;
pub mod registers;

pub use interface::EVEInterface;

pub struct EVE<I: EVEInterface> {
    ll: low_level::EVELowLevel<I>,
}

impl<I: EVEInterface> EVE<I> {
    pub fn new(ei: I) -> Self {
        Self {
            ll: low_level::EVELowLevel::new(ei),
        }
    }

    pub fn set_graphics_timings(
        &mut self,
        c: graphics_mode::EVEGraphicsTimings,
    ) -> Result<(), I::Error> {
        if c.sysclk_freq != graphics_mode::ClockFrequency::DEFAULT_SYSCLK_FREQ {}
    }
}
