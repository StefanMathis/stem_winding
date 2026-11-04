stem_winding
============

<!-- This file has ben generated with build.rs by concatenating docs/links.md,
docs/main.md and (if available docs/end.md). Do not modify this file, instead
modify the components. -->

[`Slot`]: https://docs.rs/stem_slot/0.0.1/stem_slot/slot/trait.Slot.html
[`Slot::current_displacement_coefficients]: https://docs.rs/stem_slot/0.0.1/stem_slot/slot/trait.Slot.html#tymethod.current_displacement_coefficients
[`Slot::leakage_coefficient_opening]: https://docs.rs/stem_slot/0.0.1/stem_slot/slot/trait.Slot.html#tymethod.leakage_coefficient_opening
[`RectangularSlot`]: https://docs.rs/stem_slot/0.0.1/stem_slot/rectangular/struct.RectangularSlot.html
[`OpenTrapezoidSlot`]: https://docs.rs/stem_slot/0.0.1/stem_slot/open_trapezoid/struct.OpenTrapezoidSlot.html
[`SemiTrapezoidSlot`]: https://docs.rs/stem_slot/0.0.1/stem_slot/semi_trapezoid/struct.SemiTrapezoidSlot.html
[`Slot::leakage_coefficient_opening`]: https://docs.rs/stem_slot/0.0.1/stem_slot/slot/trait.Slot.html#method.leakage_coefficient_opening
[`Slot::current_displacement_coefficients`]: https://docs.rs/stem_slot/0.0.1/stem_slot/slot/trait.Slot.html#method.current_displacement_coefficients

[double_layer_coil_layout.svg]: https://raw.githubusercontent.com/StefanMathis/stem_slot/refs/heads/main/docs/img/double_layer_coil_layout.svg
[slot_types_showcase.svg]: https://raw.githubusercontent.com/StefanMathis/stem_slot/refs/heads/main/docs/img/slot_types_showcase.svg
[current_displacement_coeffs_comp.svg]: https://raw.githubusercontent.com/StefanMathis/stem_slot/refs/heads/main/docs/img/current_displacement_coeffs_comp.svg
[magnetic_core.png]: https://raw.githubusercontent.com/StefanMathis/stem_slot/refs/heads/main/docs/img/magnetic_core.png

[![Documentation](https://docs.rs/stem_winding/badge.svg)](https://docs.rs/stem_winding)

Winding definition for stem – a Simulation Toolbox for Electric Motors. The
[stem book](https://stefanmathis.github.io/stem_book/) provides a comprehensive
guide to the framework, while this crate's documentation describes its API.

The full API documentation is available at <https://docs.rs/stem_winding/0.0.1/stem_winding>.

Winding definitions for stem - a Simulation Toolbox for Electric Motors.> **Feedback welcome!**  
> Found a bug, missing docs, or have a feature request?  
> Please open an issue on [GitHub](https://github.com/StefanMathis/stem_slot.git).

Bild End Winding (vllt. 18/4 machine)?
_Coil topology of a machine with x slots, y pole pairs and z layers, created with examples/bla.rs_

This crate is built around the [`Winding`] trait for describing and analyzing AC
windings of electrical machines.

At its core, the crate offers capabilities for machine theory analysis:
- Definition of the winding topology with [`Coil`]s
- Automated generation of [`WindingTable`]s of windings
- Analysis of winding properties such as the winding factor, the air gap leakage
factor, the field excitation curve etc, the cogging and ripple torque orders
etc.

When combined with the stem_core feature, these capabilities get extended on
real world physical property calculation:
- Phase resistance
- Inductance
- Coil extents

Further still, the cairo feature enables the visualization of the actual winding
geometry:
- Coil layout (see above)
- Winding arrangement inside the actual core geometry
Image lin core 12/5 DL tooth coil winding

A couple of predefined winding types are available: Common ones like [`DistributedWinding`],
[`ToothCoilWinding`], [`SquirrelCageWinding`], but also more exotic ones like
[`QuadrupleLayerToothCoilWinding`] or [`DistributedToothCoilWinding`] ... . If you have a winding type
which you would like to see added, write a Github issue.

# Example: Machine theory analysis

```rust
12/5 DL tooth coil winding
Showcase some things:
- Winding factor for different orders
- Harmonic orders created by the winding excitation curve
- Cogging and ripple torque orders
- Air gap leakage factor

```

# Example: Physical properties

Image 12/5 DL tooth coil winding with rotary core with open slots (MEAS servo)

```rust
Show how to calculate phase resistance, coil volume, inductance for a 12/5 DL tooth in a rotary core with open slots (MEAS servo)

```

# Acknowledgments

The technical drawings used in the docstrings have been created using 
LibreCAD (<https://librecad.org/>).
