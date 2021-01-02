#![no_std]

pub mod commands;
pub mod config;
pub mod display_list;
pub mod graphics;
pub mod interface;
pub mod low_level;
pub mod memory;
pub mod models;

/// Constructs a [`Message`](crate::strfmt::Message) value for use with EVE
/// coprocessor commands that support string formatting.
///
/// This macro understands the format syntax just enough to automatically
/// infer the types of any given arguments and thus produce a valid pairing
/// of format string and arguments. However, it achieves that by parsing the
/// format string at compile tLowLevelthe format string must always be
/// a quoted string constant.
///
/// The coprocessor's formatter serves a similar purpose as Rust's own
/// format functionality, but since the actual formatting operation happens
/// inside the EVE coprocessor we can avoid including the potentially-large
/// formatting code in memory-constrained systems. The EVE formatter can also
/// interpolate strings already stored in the EVE RAM, via the `%s` verb, which
/// Rust's own formatter doesn't have direct access to.
pub use evegfx_macros::eve_format as format;

// For more convenient use elsewhere in the crate, because we make a lot of
// use of these internally even though they are not a significant part of
// the main public interface.
pub(crate) use low_level::{host_commands, registers};

/// Model type representing the BT815 and BT816 chips.
#[doc(inline)]
pub use models::bt815::BT815;

use interface::Interface;

/// An alias for [`BT815`](BT815), because both models belong to the same
/// generation and thus share a common API.
#[doc(inline)]
pub type BT816 = BT815;

use models::Model;

/// The main type for this crate, providing a high-level API to an EVE chip
/// in terms of a low-level, platform-specific interface.
///
/// In order to interact with a real EVE chip you'll need to first select
/// an implementation of [`Interface`](interface::Interface) which you'll
/// access the chip through. This will typically be an adapter to the API
/// hardware for your platform. You can pass that interface object, along
/// with a selected model, to this type's constructor.
///
/// After instantiating an `EVE` object, the first step would typically
/// be to initialize it using its various initialization functions.
///
/// Since there are no real interface implementations in this create, the
/// following example just supposes there's already an interface in scope
/// as the variable name `ei`:
///
/// ```rust
/// # evegfx::interface::fake::interface_example(|mut ei| {
/// use evegfx::EVE;
/// let eve = EVE::new(evegfx::BT815, ei);
/// # })
/// ```
pub struct EVE<M: Model, I: Interface> {
    pub(crate) ll: low_level::LowLevel<M, I>,
}

impl<M: Model, I: Interface> EVE<M, I> {
    /// Construct a new `EVE` object for the given EVE model, communicating
    /// via the given interface.
    ///
    /// Models are represented by empty struct types in this crate, such
    /// as [`BT815`](BT815) for the BT815 and BT816 models. The different
    /// models all have a broadly-compatible API but later generations have
    /// additional functionality.
    #[allow(unused_variables)]
    pub fn new(m: M, ei: I) -> Self {
        Self::new_internal(ei)
    }

    // This is an internal version of `new` for situations where type inference
    // already implies a particular model type.
    pub(crate) fn new_internal(ei: I) -> Self {
        Self {
            ll: low_level::LowLevel::new(ei),
        }
    }

    /// Consumes the `EVE` object and returns its underlying interface.
    pub fn take_interface(self) -> I {
        self.ll.take_interface()
    }

    pub fn borrow_interface<'a>(&'a mut self) -> &'a mut I {
        self.ll.borrow_interface()
    }

    /// Consume the `EVE` object and returns an instance of `LowLevel`
    /// that uses the same interface.
    pub fn take_low_level(self) -> low_level::LowLevel<M, I> {
        self.ll
    }

    pub fn borrow_low_level<'a>(&'a mut self) -> &'a mut low_level::LowLevel<M, I> {
        &mut self.ll
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
        source: config::ClockSource,
        video: &config::VideoTimings,
    ) -> Result<(), I::Error> {
        config::activate_system_clock(self, source, video)
    }

    /// Busy-waits while polling the EVE ID for its ID register. Once it
    /// returns the expected value that indicates that the boot process
    /// is complete and this function will return.
    ///
    /// If the connected device isn't an EVE, or if the chip isn't connected
    /// correctly, or if it's failing boot in some other way then this
    /// function will poll forever.
    pub fn poll_for_boot(&mut self, poll_limit: u32) -> Result<bool, I::Error> {
        config::poll_for_boot(self, poll_limit)
    }

    pub fn configure_video_pins(
        &mut self,
        mode: &config::RGBElectricalMode,
    ) -> Result<(), I::Error> {
        config::configure_video_pins(self, mode)
    }

    /// Configures registers to achieve a particular graphics mode wLowLevel
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
    pub fn start_video(&mut self, c: &config::VideoTimings) -> Result<(), I::Error> {
        config::activate_pixel_clock(self, c)
    }

    pub fn new_display_list<
        F: FnOnce(&mut display_list::JustBuilder<low_level::LowLevel<M, I>>) -> Result<(), I::Error>,
    >(
        &mut self,
        f: F,
    ) -> Result<(), I::Error> {
        self.ll.dl_reset();
        {
            let mut builder = display_list::just_builder(&mut self.ll);
            f(&mut builder)?;
        }
        let dlswap_ptr = M::reg_ptr(registers::Register::DLSWAP);
        self.ll.wr8(dlswap_ptr, 0b00000010)
    }

    /// Consumes the main EVE object and returns an interface to the
    /// coprocessor component of the chip, using the given waiter to pause
    /// when the command buffer becomes too full.
    ///
    /// The typical way to use an EVE device is to initialize it via direct
    /// register writes and then do all of the main application activities
    /// via the coprocessor, which exposes all of the system's capabilities
    /// either directly or indirectly.
    pub fn coprocessor<W: commands::waiter::Waiter<M, I>>(
        self,
        waiter: W,
    ) -> Result<commands::Coprocessor<M, I, W>, commands::Error<I::Error, W::Error>> {
        let ei = self.ll.take_interface();
        commands::Coprocessor::new(ei, waiter)
    }

    /// A wrapper around `coprocessor` which automatically provides a
    /// busy-polling waiter. This can be a good choice particularly if your
    /// application typically generates coprocessor commands slow enough that
    /// the buffer will rarely fill, and thus busy waiting will not be
    /// typical.
    ///
    /// However, this will use more CPU and cause more SPI traffic than an
    /// interrupt-based waiter for applications that frequently need to wait
    /// for command processing, such as those which attempt to synchronize
    /// with the display refresh rate and thus could often end up waiting for
    /// the scanout to "catch up".
    pub fn coprocessor_polling(
        self,
    ) -> commands::Result<
        commands::Coprocessor<M, I, commands::waiter::PollingWaiter<M, I>>,
        M,
        I,
        commands::waiter::PollingWaiter<M, I>,
    > {
        let ei = self.ll.take_interface();
        commands::Coprocessor::new_polling(ei)
    }
}
