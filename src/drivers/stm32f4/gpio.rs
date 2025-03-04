//! Macro-instantiated GPIO implementation.
//!
//! Pin configuration is encoded in the type system through typestates,
//! making it statically impossible to misuse a pin (e.g. there's
//! no "write" operation on a pin that has been configured as input).
#![macro_use]

use core::marker::PhantomData;

/// Input mode (Pin type state)
pub struct Input<MODE> {
    // NOTE: The role of PhantomData is to represent that
    // this Input typestate "owns" a generic MODE typestate,
    // establishing a typestate hierarchy. Other usages of
    // PhantomData in this file are similar.
    _mode: PhantomData<MODE>,
}

/// Floating input (Input type state)
pub struct Floating;
/// Pulled down input (Input type state)
pub struct PullDown;
/// Pulled up input (Input type state)
pub struct PullUp;

/// Output mode (Pin type state)
pub struct Output<MODE> {
    _mode: PhantomData<MODE>,
}
/// Push pull output (Output type state)
pub struct PushPull;
/// Open drain output (Output type state)
pub struct OpenDrain;

#[macro_export]
macro_rules! enable_gpio {
    () => {
        /// Extension trait to split a GPIO peripheral in independent pins and registers
        pub trait GpioExt {
            /// The type to split the GPIO into
            type GpioWrapper;

            /// Splits the GPIO block into independent pins and registers
            fn split(self, rcc: &mut blue_hal::stm32pac::RCC) -> Self::GpioWrapper;
        }

        pin_rows!(a, b, c, d, e, f, g, h, i, j, k,);
        alternate_functions!(0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,);
        enable_qspi!();
        enable_spi!();
        enable_serial!();
    };
}

#[allow(unused)]
#[macro_export(local_inner_macros)]
macro_rules! seal_pins { ($function:ty: [$($pin:ty,)+]) => {
    $(
        unsafe impl $function for $pin {}
    )+
};}

// Typestate generator for all Alternate Functions
#[macro_export(local_inner_macros)]
macro_rules! alternate_functions {
    ($($i:expr, )+) => { $( $crate::paste::item! {
        /// Alternate function (Pin type state)
        pub struct [<AF $i>];
    } )+ }
}

// Type generator for all pins
#[macro_export(local_inner_macros)]
macro_rules! pin_rows {
    ($($x:ident,)+) => {
        $(
            pin_row!($x, [0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,]);
        )+
    }
}

#[allow(unused)]
#[macro_export(local_inner_macros)]
macro_rules! pin_row {
    ($x:ident, [$($i:expr,)+]) => { $( $crate::paste::item! {
        /// Pin with a MODE typestate
        pub struct [<P $x $i>]<MODE> {
            _mode: core::marker::PhantomData<MODE>,
        }
    } )+
    }
}

/// Instantiates a gpio pin row with default modes per available pin.
///
/// # Examples
///
/// ```ignore
///   gpio!(b, [
///      (7, Output::<PushPull>),
///      (8, AF4),
///      (3, Input::<Floating>),
///   ]);
///
/// ```
/// This makes the wrapper struct gpiob have the members gpiob.pb7 in
/// Output + Push/Pull mode, gpiob.pb8 in alternate function 4, and
/// gpiob.pb3 as a floating input.
#[macro_export]
macro_rules! gpio {
    ($x: ident, [
        $( ($i:expr, $default_mode:ty $(as $function:ident$(<$T:ident>)?)?), )*
    ]) => {

        // Macro black magic. the "paste" crate generates a context where anything bounded by "[<"
        // and ">]" delimiters gets concatenated in a single identifier post macro expansion. For
        // example, "[<GPIO $x>]" becomes "GPIOa" when "$x" represents "a". This is used to
        // expand the outer level, simplified "gpio!" instantiation macro into the complex one.
        $crate::paste::item! {
            gpio_inner!([<GPIO $x>], [<gpio $x>], [<gpio $x en>], [<gpio $x rst>], [<P $x x>], [
                $( [<P $x $i>]: ([<p $x $i>], $i, $default_mode, $([<Earmark $x $i>], $function$(<$T>)?)?), )*
            ]);
        }
    }
}

#[macro_export(local_inner_macros)]
macro_rules! into_af {
    ($GPIOx:ident, $i:expr, $Pxi:ident, $pxi:ident, [$($af_i:expr, )+]) => { $( $crate::paste::item! {
        pub fn [<into_af $af_i>](self) -> $Pxi<[<AF $af_i>]> {
            let offset = 2 * $i;

            // alternate function mode
            let mode = 0b10;

            // NOTE(safety) atomic read-modify-write operation to a stateless register.
            // It is also safe because pins are only reachable by splitting a GPIO struct,
            // which preserves single ownership of each pin.
            unsafe {
                (*$GPIOx::ptr()).moder.modify(|r, w|
                    w.bits((r.bits() & !(0b11 << offset)) | (mode << offset))
                );
            }

            let offset = 4 * ($i % 8);

            if $i < 8 {
                // NOTE(safety) atomic read-modify-write operation to a stateless register.
                // It is also safe because pins are only reachable by splitting a GPIO struct,
                // which preserves single ownership of each pin.
                unsafe {
                    (*$GPIOx::ptr()).afrl.modify(|r, w|
                        w.bits((r.bits() & !(0b1111 << offset)) | ($af_i << offset))
                    );
                }
            } else {
                // NOTE(safety) atomic read-modify-write operation to a stateless register.
                // It is also safe because pins are only reachable by splitting a GPIO struct,
                // which preserves single ownership of each pin.
                unsafe {
                    (*$GPIOx::ptr()).afrh.modify(|r, w|
                        w.bits((r.bits() & !(0b1111 << offset)) | ($af_i << offset))
                    );
                }
            }

            $Pxi { _mode: PhantomData }
        }
} )+ }
}

#[macro_export(local_inner_macros)]
macro_rules! new_af {
    ($GPIOx:ident, $i:expr, $Pxi:ident, $pxi:ident, [$($af_i:expr, )+]) => { $( $crate::paste::item! {
        impl $Pxi<[<AF $af_i>]> {
            #[allow(dead_code)]
            fn new() -> Self {
                let pin = $Pxi::<Input<Floating>> { _mode : PhantomData };
                pin.[<into_af $af_i>]()
            }
        }
} )+ }
}

#[macro_export(local_inner_macros)]
macro_rules! gpio_inner {
    ($GPIOx:ident, $gpiox:ident, $enable_pin:ident, $reset_pin:ident, $Pxx:ident, [
        $($Pxi:ident: ($pxi:ident, $i:expr, $default_mode:ty, $($earmark:ident, $function:ident$(<$T:ident>)?)?), )*
    ]) => {

        /// GPIO
        pub mod $gpiox {
            use core::marker::PhantomData;
            use blue_hal::hal::gpio::{OutputPin, InputPin};
            use super::*;

            // Lower case for identifier concatenation
            #[allow(unused_imports)]
            use blue_hal::stm32pac::{
                GPIOA as GPIOa,
                GPIOB as GPIOb,
                GPIOC as GPIOc,
                GPIOD as GPIOd,
                GPIOE as GPIOe,
                GPIOF as GPIOf,
                GPIOG as GPIOg,
                GPIOH as GPIOh,
            };

            #[allow(unused_imports)]
            #[cfg(not(feature = "stm32f412"))]
            use blue_hal::stm32pac::{
                GPIOI as GPIOi,
                GPIOJ as GPIOj,
                GPIOK as GPIOk,
            };

            use blue_hal::drivers::stm32f4::gpio::*;

            /// GPIO parts
            pub struct GpioWrapper {
                $(
                    /// Pin
                    pub $pxi: $Pxi<$default_mode>,
                )*
            }

            impl GpioExt for $GPIOx {
                type GpioWrapper = GpioWrapper;

                fn split(self, rcc: &mut blue_hal::stm32pac::RCC) -> GpioWrapper {
                    rcc.ahb1enr.modify(|_, w| w.$enable_pin().enabled());
                    rcc.ahb1rstr.modify(|_, w| w.$reset_pin().set_bit());
                    rcc.ahb1rstr.modify(|_, w| w.$reset_pin().clear_bit());

                    $(
                        let $pxi = $Pxi::<$default_mode>::new();
                    )*

                    GpioWrapper {
                        $($pxi,)*
                    }
                }
            }
            /// Partially erased pin
            pub struct $Pxx<MODE> {
                i: u8,
                _mode: PhantomData<MODE>,
            }

            impl<MODE> OutputPin for $Pxx<Output<MODE>> {
                fn set_high(&mut self) {
                    // NOTE(safety) atomic write to a stateless register. It is also safe
                    // because pins are only reachable by splitting a GPIO struct,
                    // which preserves single ownership of each pin.
                    unsafe { (*$GPIOx::ptr()).bsrr.write(|w| w.bits(1 << self.i)) }
                }

                fn set_low(&mut self) {
                    // NOTE(safety) atomic write to a stateless register. It is also safe
                    // because pins are only reachable by splitting a GPIO struct,
                    // which preserves single ownership of each pin.
                    unsafe { (*$GPIOx::ptr()).bsrr.write(|w| w.bits(1 << (16 + self.i))) }
                }
            }

            impl<MODE> InputPin for $Pxx<Input<MODE>> {
                fn is_high(&self) -> bool {
                    // NOTE(safety) atomic read from a stateless register. It is also safe
                    // because pins are only reachable by splitting a GPIO struct,
                    // which preserves single ownership of each pin.
                    unsafe { (((*$GPIOx::ptr()).idr.read().bits() >> self.i) & 0b1) != 0 }
                }

                fn is_low(&self) -> bool{
                    !self.is_high()
                }
            }

            $(
                /// Pin
                impl $Pxi<Input<Floating>> {
                    #[allow(dead_code)]
                    fn new() -> Self {
                        $Pxi { _mode: PhantomData }
                    }
                }

                impl $Pxi<Output<PushPull>> {
                    #[allow(dead_code)]
                    fn new() -> Self {
                        let pin = $Pxi::<Input<Floating>> { _mode : PhantomData };
                        pin.into_push_pull_output()
                    }
                }

                impl $Pxi<Input<PullDown>> {
                    #[allow(dead_code)]
                    fn new() -> Self {
                        let pin = $Pxi::<Input<Floating>> { _mode : PhantomData };
                        pin.into_pull_down_input()
                    }
                }

                impl $Pxi<Input<PullUp>> {
                    #[allow(dead_code)]
                    fn new() -> Self {
                        let pin = $Pxi::<Input<Floating>> { _mode : PhantomData };
                        pin.into_pull_up_input()
                    }
                }

                impl $Pxi<Output<OpenDrain>> {
                    #[allow(dead_code)]
                    fn new() -> Self {
                        let pin = $Pxi::<Input<Floating>> { _mode : PhantomData };
                        pin.into_open_drain_output()
                    }
                }

                new_af!($GPIOx, $i, $Pxi, $pxi, [0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,]);

                $(
                    // If this field exists, it means the gpio has been earmarked for
                    // a particular purpose in the gpio table.
                    trait $earmark: $function$(<$T>)? {}
                    impl $earmark for $Pxi<$default_mode> {}
                )?

                impl<MODE> $Pxi<MODE> {
                    into_af!($GPIOx, $i, $Pxi, $pxi, [0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,]);

                    /// Configures the pin to operate as a floating input pin
                    pub fn into_floating_input(
                        self,
                    ) -> $Pxi<Input<Floating>> {
                        let offset = 2 * $i;

                        // input mode
                        // NOTE(safety) atomic read-modify-write operation to a stateless register.
                        // It is also safe because pins are only reachable by splitting a GPIO struct,
                        // which preserves single ownership of each pin.
                        unsafe { (*$GPIOx::ptr()).moder.modify(|r, w| w.bits(r.bits() & !(0b11 << offset)) ); }

                        // no pull-up or pull-down
                        // NOTE(safety) atomic read-modify-write operation to a stateless register.
                        // It is also safe because pins are only reachable by splitting a GPIO struct,
                        // which preserves single ownership of each pin.
                        unsafe { (*$GPIOx::ptr()).pupdr.modify(|r, w|  w.bits(r.bits() & !(0b11 << offset)) ); }

                        $Pxi { _mode: PhantomData }
                    }

                    /// Configures the pin to operate as a pulled down input pin
                    pub fn into_pull_down_input(
                        self,
                    ) -> $Pxi<Input<PullDown>> {
                        let offset = 2 * $i;

                        // input mode
                        // NOTE(safety) atomic read-modify-write operation to a stateless register.
                        // It is also safe because pins are only reachable by splitting a GPIO struct,
                        // which preserves single ownership of each pin.
                        unsafe { (*$GPIOx::ptr()).moder.modify(|r, w| w.bits(r.bits() & !(0b11 << offset)) ); }

                        // pull-down
                        // NOTE(safety) atomic read-modify-write operation to a stateless register.
                        // It is also safe because pins are only reachable by splitting a GPIO struct,
                        // which preserves single ownership of each pin.
                        unsafe { (*$GPIOx::ptr()).pupdr.modify(|r, w|
                            w.bits((r.bits() & !(0b11 << offset)) | (0b10 << offset))
                        ); }

                        $Pxi { _mode: PhantomData }
                    }

                    /// Configures the pin to operate as a pulled up input pin
                    pub fn into_pull_up_input(
                        self,
                    ) -> $Pxi<Input<PullUp>> {
                        let offset = 2 * $i;

                        // input mode
                        // NOTE(safety) atomic read-modify-write operation to a stateless register.
                        // It is also safe because pins are only reachable by splitting a GPIO struct,
                        // which preserves single ownership of each pin.
                        unsafe { (*$GPIOx::ptr()).moder.modify(|r, w| w.bits(r.bits() & !(0b11 << offset)) ); }

                        // pull-up
                        // NOTE(safety) atomic read-modify-write operation to a stateless register.
                        // It is also safe because pins are only reachable by splitting a GPIO struct,
                        // which preserves single ownership of each pin.
                        unsafe { (*$GPIOx::ptr()).pupdr.modify(|r, w|
                            w.bits((r.bits() & !(0b11 << offset)) | (0b01 << offset))
                        ); }

                        $Pxi { _mode: PhantomData }
                    }

                    /// Configures the pin to operate as an open drain output pin
                    pub fn into_open_drain_output(
                        self,
                    ) -> $Pxi<Output<OpenDrain>> {
                        let offset = 2 * $i;

                        // general purpose output mode
                        let mode = 0b01;
                        // NOTE(safety) atomic read-modify-write operation to a stateless register.
                        // It is also safe because pins are only reachable by splitting a GPIO struct,
                        // which preserves single ownership of each pin.
                        unsafe { (*$GPIOx::ptr()).moder.modify(|r, w|
                            w.bits((r.bits() & !(0b11 << offset)) | (mode << offset))
                        ); }

                        // open drain output
                        // NOTE(safety) atomic read-modify-write operation to a stateless register.
                        // It is also safe because pins are only reachable by splitting a GPIO struct,
                        // which preserves single ownership of each pin.
                        unsafe { (*$GPIOx::ptr()).otyper.modify(|r, w| w.bits(r.bits() | (0b1 << $i)) ); }

                        $Pxi { _mode: PhantomData }
                    }

                    /// Configures the pin to operate as an push pull output pin
                    pub fn into_push_pull_output(
                        self,
                    ) -> $Pxi<Output<PushPull>> {
                        let offset = 2 * $i;

                        // general purpose output mode
                        let mode = 0b01;

                        // NOTE(safety) atomic read-modify-write operation to a stateless register.
                        // It is also safe because pins are only reachable by splitting a GPIO struct,
                        // which preserves single ownership of each pin.
                        unsafe { (*$GPIOx::ptr()).moder.modify(|r, w|
                            w.bits((r.bits() & !(0b11 << offset)) | (mode << offset))
                        ); }

                        // push pull output
                        // NOTE(safety) atomic read-modify-write operation to a stateless register.
                        // It is also safe because pins are only reachable by splitting a GPIO struct,
                        // which preserves single ownership of each pin.
                        unsafe { (*$GPIOx::ptr()).otyper.modify(|r, w| w.bits(r.bits() & !(0b1 << $i)) ); }

                        $Pxi { _mode: PhantomData }
                    }
                }

                impl $Pxi<Output<OpenDrain>> {
                    /// Enables / disables the internal pull up
                    pub fn internal_pull_up(&mut self, on: bool) {
                        let offset = 2 * $i;

                        // NOTE(safety) atomic read-modify-write operation to a stateless register.
                        // It is also safe because pins are only reachable by splitting a GPIO struct,
                        // which preserves single ownership of each pin.
                        unsafe { (*$GPIOx::ptr()).pupdr.modify(|r, w|
                            w.bits(
                                (r.bits() & !(0b11 << offset)) | if on {
                                    0b01 << offset
                                } else {
                                    0
                                },
                            )
                        ); }
                    }
                }

                impl<MODE> $Pxi<Output<MODE>> {
                    /// Erases the pin number from the type
                    ///
                    /// This is useful when you want to collect the pins into an array where you
                    /// need all the elements to have the same type
                    pub fn downgrade(self) -> $Pxx<Output<MODE>> {
                        $Pxx {
                            i: $i,
                            _mode: self._mode,
                        }
                    }
                }

                impl<MODE> OutputPin for $Pxi<Output<MODE>> {
                    fn set_high(&mut self) {
                        // NOTE(safety) atomic write to a stateless register. It is also safe
                        // because pins are only reachable by splitting a GPIO struct,
                        // which preserves single ownership of each pin.
                        unsafe { (*$GPIOx::ptr()).bsrr.write(|w| w.bits(1 << $i)) }
                    }

                    fn set_low(&mut self) {
                        // NOTE(safety) atomic write to a stateless register. It is also safe
                        // because pins are only reachable by splitting a GPIO struct,
                        // which preserves single ownership of each pin.
                        unsafe { (*$GPIOx::ptr()).bsrr.write(|w| w.bits(1 << (16 + $i))) }
                    }
                }

            impl<MODE> InputPin for $Pxi<Input<MODE>> {
                fn is_high(&self) -> bool {
                    // NOTE(safety) atomic read from a stateless register. It is also safe
                    // because pins are only reachable by splitting a GPIO struct,
                    // which preserves single ownership of each pin.
                    unsafe { (((*$GPIOx::ptr()).idr.read().bits() >> $i) & 0b1) != 0 }
                }

                fn is_low(&self) -> bool{
                    !self.is_high()
                }
            }
            )*
        }
    }
}
