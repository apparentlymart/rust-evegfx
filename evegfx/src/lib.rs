#![no_std]

pub mod color;
pub mod display_list;
pub mod graphics_mode;
pub mod host_commands;
pub mod init;
pub mod interface;
pub mod low_level;
pub mod registers;

pub use graphics_mode::{EVEGraphicsTimings, EVERGBElectricalMode};
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
    /// process. You could use the `poll_for_boot` method as an easy way to
    /// wait for the device to become ready, albeit via busy-waiting.
    ///
    /// The typical next steps are to first call `configure_video_pins` in
    /// order to configure the physical characteristics of the Parallel RGB
    /// interface, and then to call `start_video` to configure the video mode
    /// and activate the pixel clock.
    pub fn start_system_clock(
        &mut self,
        source: init::EVEClockSource,
        mode: graphics_mode::EVEGraphicsTimings,
    ) -> Result<(), I::Error> {
        init::activate_system_clock(self, source, mode)
    }

    /// Busy-waits while polling the EVE ID for its ID register. Once it
    /// returns the expected value that indicates that the boot process
    /// is complete and this function will return.
    ///
    /// If the connected device isn't an EVE, or if the chip isn't connected
    /// correctly, or if it's failing boot in some other way then this
    /// function will poll forever.
    pub fn poll_for_boot(&mut self, poll_limit: u32) -> Result<bool, I::Error> {
        init::poll_for_boot(self, poll_limit)
    }

    pub fn configure_video_pins(
        &mut self,
        mode: graphics_mode::EVERGBElectricalMode,
    ) -> Result<(), I::Error> {
        init::configure_video_pins(self, mode)
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

    pub fn new_display_list<
        F: FnOnce(&mut display_list::DLBuilder<low_level::EVELowLevel<I>>) -> Result<(), I::Error>,
    >(
        &mut self,
        f: F,
    ) -> Result<(), I::Error> {
        self.ll.dl_reset();
        let mut builder = display_list::DLBuilder::new(&mut self.ll);
        f(&mut builder)
    }
}
