#![no_std]

pub mod display_list;
pub mod graphics_mode;
pub mod host_commands;
pub mod init;
pub mod interface;
pub mod low_level;
pub mod registers;

pub use graphics_mode::EVEGraphicsTimings;
pub use init::EVEClockSource;
pub use interface::EVEInterface;

pub struct EVE<I: EVEInterface> {
    pub(crate) ll: low_level::EVELowLevel<I>,
}

impl<I: EVEInterface> EVE<I> {
    pub fn new(ei: I) -> Self {
        Self {
            ll: low_level::EVELowLevel::new(ei),
        }
    }

    /// Sends commands to the device to configure and then activate the system
    /// clock.
    ///
    /// If this function succeeds then the system clock will be activated and
    /// the device will have begun (but not necessarily completed) its boot
    /// process. The next step is usually to call `set_graphics_timings`
    /// in order to configure the graphics mode and activate the video output.
    pub fn start_system_clock<'a>(
        &'a mut self,
        source: init::EVEClockSource,
        mode: graphics_mode::EVEGraphicsTimings,
    ) -> Result<(), I::Error> {
        init::activate_system_clock(self, source, mode)
    }

    /// Configures registers to achieve a particular graphics mode with the
    /// given timings.
    ///
    /// You should typically call `start_system_clock` first, using the
    /// same timing value, in order to activate the system clock. Calling
    /// `start_video` afterwards will then activate the graphics engine with
    /// a pixel clock derived from the system clock.
    ///
    /// If you call `start_video` with a different timings value than you most
    /// recently passed to `start_system_clock` then this is likely to
    /// produce an invalid video signal.
    ///
    /// If this function succeeds then the display will be active before it
    /// returns, assuming that the chip itself was already activated.
    pub fn start_video(&mut self, c: graphics_mode::EVEGraphicsTimings) -> Result<(), I::Error> {
        init::activate_pixel_clock(self, c)
    }
}
