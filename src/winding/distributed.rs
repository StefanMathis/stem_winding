//! Provides the [`DistributedWinding`] as well as its builder structs.
//!
//! See the documentation of [`DistributedWinding`] for details.

use std::num::{NonZeroU16, NonZeroUsize};

use compare_variables::{Comparison, compare_variables};

use dyn_clone::clone_box;
use num::rational::Ratio;

use stem_types::{CoilLayout, Zone};
use stem_wire::prelude::*;

#[cfg(feature = "stem_core")]
use stem_core::prelude::*;

#[cfg(feature = "stem_core")]
use crate::core_support::*;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::{
    coils::{Coil, Coils, FullCoil},
    error::{Error, WindingTableConstructionError},
    winding::{Connection, Winding, base_winding_count_symmetric, hole_number},
    winding_table::{WindingTable, WindingTableConstructor},
};

/**
Tries to create a new instange of an `DistributedWinding`. Fails if the combination of input parameters does not lead to a legal winding.

Arguments:
 - `concentric_coils`: If `true`, the coils of a coil group are constructed from concentric coils, otherwise each coil of a group has the same span.

Example concentric coils:
```text
┌─────────────────┐
│ ┌────────────┐  │
1 1 -3 -3 2 2 -1 -1
│ └────────────┘  │
└─────────────────┘
```

Example coils with same span:
```text
┌──────────────┐
│ ┌────────────│──┐
1 1 -3 -3 2 2 -1 -1
│ └────────────│──┘
└──────────────┘
```
 */
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct DistributedWinding {
    slots: NonZeroU16,
    pole_pairs: NonZeroU16,
    phases: NonZeroU16,
    layers: NonZeroU16,
    coil_throw_reduction: i32,
    zone_shift: u16,
    turns_per_coil: NonZeroUsize,
    parallel_paths: NonZeroU16,
    connection: Connection,
    end_winding_leakage_coefficient: f64,
    wire: Box<dyn Wire>,
    #[cfg_attr(feature = "serde", serde(skip))]
    coils: Coils,
    winding_table_constructor: WindingTableConstructor,
    concentric_coils: bool,
    mean_end_winding_half_turn_length: Option<Length>,
}

impl DistributedWinding {
    /// Creates a new winding.
    ///
    /// This is a wrapper around `T::try_into`. The builder structs provided by
    /// the module implement [`TryInto<DistributedWinding>`], see their
    /// respective documentation for details on the failure modes. The
    /// struct documentation has an usage example.
    pub fn new<W>(builder: W) -> Result<Self, Error>
    where
        W: TryInto<DistributedWinding>,
        W::Error: Into<Error>,
    {
        builder.try_into().map_err(Into::into)
    }

    /// Returns a decomposition of the
    /// [`winding_factor`](Winding::winding_factor) into the "distribution
    /// factor" and the "pitch factor".
    ///
    /// If a double-layer winding is not full-pitched
    /// ([`coil_throw_reduction`](DistributedWinding::coil_throw_reduction)
    /// unequal zero), the winding factor `k_w,v` can be decomposed into a
    /// component stemming from the distribution of a phase over multiple slots
    /// ("distribution factor" `k_d,v`) and the influence of the pitching
    /// ("pitch factor" `k_p,v`):
    ///
    /// `k_w,v = k_d,v * k_p,v`.
    ///
    /// See e.g. [\[1\]](#distribution_and_pitch_factor_1), eq. 3.2-15 and
    /// 3.2-17 for details.
    ///
    /// This function returns the two-value array `[distribution_factor,
    /// pitch_factor]`.
    ///
    /// # Literature
    /// <a id="distribution_and_pitch_factor_1">\[1\]</a>
    /// Binder, A.: Elektrische Maschinen und Antriebe, 1st edition, Springer
    /// Heidelberg, 2012
    ///
    /// # Examples
    ///
    /// ## Full-pitched winding
    ///
    /// The pitch factor is one and subsequently, `k_d == k_w`.
    ///
    /// ```
    /// use std::num::NonZeroU16;
    /// use approxim::assert_abs_diff_eq;
    /// use stem_winding::prelude::*;
    ///
    /// let full_pitched = DistributedWinding::new(DistributedMinimalBuilder {
    ///     slots: 12.try_into().expect("not zero"),
    ///     pole_pairs: 1.try_into().expect("not zero"),
    ///     phases: 3.try_into().expect("not zero"),
    ///     layers: 2.try_into().expect("not zero"),
    ///     winding_table_constructor: WindingTableConstructor::Tingley,
    ///     coil_throw_reduction: 0,
    ///     zone_shift: 0,
    /// }).expect("valid winding topology");
    /// let ph = NonZeroU16::new(1).expect("not zero");
    ///
    /// let [k_d, k_p] = full_pitched.distribution_and_pitch_factor(ph, SpatialOrder::Electrical(1));
    ///
    /// assert_abs_diff_eq!(k_d, 0.9659, epsilon = 1e-3);
    /// assert_abs_diff_eq!(k_p, 1.0, epsilon = 1e-3);
    ///
    /// let k_w = full_pitched.winding_factor(ph, SpatialOrder::Electrical(1));
    /// assert_abs_diff_eq!(k_w, k_d * k_p, epsilon = 1e-3);
    /// ```
    ///
    /// ## Short-pitched winding
    ///
    /// The pitch factor is smaller than one and subsequently, `k_d > k_w`.
    ///
    /// ```
    /// use std::num::NonZeroU16;
    /// use approxim::assert_abs_diff_eq;
    /// use stem_winding::prelude::*;
    ///
    /// let short_pitched = DistributedWinding::new(DistributedMinimalBuilder {
    ///     slots: 12.try_into().expect("not zero"),
    ///     pole_pairs: 1.try_into().expect("not zero"),
    ///     phases: 3.try_into().expect("not zero"),
    ///     layers: 2.try_into().expect("not zero"),
    ///     winding_table_constructor: WindingTableConstructor::Tingley,
    ///     coil_throw_reduction: 1,
    ///     zone_shift: 0,
    /// }).expect("valid winding topology");
    /// let ph = NonZeroU16::new(1).expect("not zero");
    ///
    /// let [k_d, k_p] = short_pitched.distribution_and_pitch_factor(ph, SpatialOrder::Electrical(1));
    ///
    /// assert_abs_diff_eq!(k_d, 0.9659, epsilon = 1e-3);
    /// assert_abs_diff_eq!(k_p, 0.9659, epsilon = 1e-3);
    ///
    /// let k_w = short_pitched.winding_factor(ph, SpatialOrder::Electrical(1));
    /// assert_abs_diff_eq!(k_w, k_d * k_p, epsilon = 1e-3);
    /// ```
    pub fn distribution_and_pitch_factor(
        &self,
        phase: NonZeroU16,
        spatial_order: SpatialOrder,
    ) -> [f64; 2] {
        // If layers==1, the distribution factor equals the winding factor,
        // meaning that the pitch factor is 1 by default. If layers == 2,
        // distribution and pitch factor are calculated by temporarily changing the
        // pitch to zero.
        let coil_throw_reduction = self.coil_throw_reduction();
        if self.layers().get() == 1 || coil_throw_reduction == 0 {
            return [self.winding_factor(phase, spatial_order), 1.0];
        } else {
            // Create a copy of this winding, but without the short pitch. We use
            // the DistributionTable here, since it can deal with all slot / pole
            // combinations as long as there are one or two layers (which is
            // always the case for a DistributedWinding).
            let non_reduced: DistributedWinding = DistributedMinimalBuilder {
                slots: self.slots(),
                pole_pairs: self.pole_pairs(),
                phases: self.phases(),
                layers: self.layers(),
                coil_throw_reduction: 0,
                zone_shift: self.zone_shift,
                winding_table_constructor: self.winding_table_constructor,
            }
            .try_into()
            .expect("all input parameters result in a valid winding, since self is valid as well");

            // Calculate winding factor with and without chording
            let k_d = non_reduced.winding_factor(phase, spatial_order);
            let k_w_chorded = self.winding_factor(phase, spatial_order);

            // The pitch factor is calculated as the quotient of factor_w_chording and
            // factor_wo_chording
            let k_p = k_w_chorded / k_d;
            return [k_d, k_p];
        }
    }

    /// Returns the coil-throw reduction for a double-layer winding in slots.
    /// For a single-layer winding, this value is meaningless.
    ///
    /// A positive value indicates short-pitched coils; a negative value
    /// indicates long-pitched coils. The coil-throw reduction is defined as
    ///
    /// `coil-throw reduction = full-pitch throw - actual coil throw`
    ///
    /// where the full-pitch throw is `slots / (2 * pole pairs)`.
    ///
    /// Short-pitching is mainly done for the following two reasons:
    /// 1) It reduces the end winding length and therefore resistance, copper
    ///    usage etc.
    /// 2) Due to the zone shift, the winding factor is influenced. This effect
    ///    can be used to suppress unwanted harmonics, as demonstrated in the
    ///    example section. For example, a 12-slot, 2-pole full-pitched winding
    ///    has a fundamental winding factor of 0.9659 and that of its fifth
    ///    electrical spatial order is 0.2588. Reducing the coil-throw by one
    ///    slot results in a fundamental winding factor of 0.9330, whereas that
    ///    of the fifth electrical spatial order drops to 0.0669 - a fourfold
    ///    reduction at the price of a 3.4 % reduction of the fundamental
    ///    winding factor.
    ///
    /// # Examples
    ///
    /// The following examples compare a full-pitched with a short-pitched
    /// winding for the aforementioned 12-slot, 2-pole topology.
    ///
    /// ## Full-pitched
    #[doc = ""]
    #[cfg_attr(
        feature = "doc-images",
        doc = "![Full-pitched 12/2 distributed winding][table_distributed_winding_red_0]"
    )]
    #[cfg_attr(
        feature = "doc-images",
        embed_doc_image::embed_doc_image(
            "table_distributed_winding_red_0",
            "docs/img/table_distributed_winding_red_0.svg"
        )
    )]
    #[cfg_attr(
        not(feature = "doc-images"),
        doc = "**Doc images not enabled**. Compile docs with
    `cargo doc --features 'doc-images'` and Rust version >= 1.54."
    )]
    /// ```
    /// use std::num::NonZeroU16;
    /// use approxim::assert_abs_diff_eq;
    /// use stem_winding::prelude::*;
    ///
    /// let full_pitched = DistributedWinding::new(DistributedMinimalBuilder {
    ///     slots: 12.try_into().expect("not zero"),
    ///     pole_pairs: 1.try_into().expect("not zero"),
    ///     phases: 3.try_into().expect("not zero"),
    ///     layers: 2.try_into().expect("not zero"),
    ///     winding_table_constructor: WindingTableConstructor::Tingley,
    ///     coil_throw_reduction: 0,
    ///     zone_shift: 0,
    /// }).expect("valid winding topology");
    ///
    /// // Coil span is 6 (slots / (2 * pole_pairs))
    /// assert_eq!(
    ///     full_pitched.coil_at([0, 0].into()).expect("exists").throw(Some(full_pitched.slots())),
    ///     6
    /// );
    ///
    /// // Winding factor for the fundamental and the first non-zero superharmonic
    /// let ph = NonZeroU16::new(1).expect("not zero");
    /// assert_abs_diff_eq!(
    ///     full_pitched.winding_factor(ph, SpatialOrder::Electrical(1)),
    ///     0.9659,
    ///     epsilon = 1e-3
    /// );
    /// assert_abs_diff_eq!(
    ///     full_pitched.winding_factor(ph, SpatialOrder::Electrical(5)),
    ///     0.2588,
    ///     epsilon = 1e-3
    /// );
    /// ```
    /// ## Short-pitched
    #[doc = ""]
    #[cfg_attr(
        feature = "doc-images",
        doc = "![Short-pitched 12/2 distributed winding][table_distributed_winding_red_1]"
    )]
    #[cfg_attr(
        feature = "doc-images",
        embed_doc_image::embed_doc_image(
            "table_distributed_winding_red_1",
            "docs/img/table_distributed_winding_red_1.svg"
        )
    )]
    #[cfg_attr(
        not(feature = "doc-images"),
        doc = "**Doc images not enabled**. Compile docs with
    `cargo doc --features 'doc-images'` and Rust version >= 1.54."
    )]
    /// ```
    /// use std::num::NonZeroU16;
    /// use approxim::assert_abs_diff_eq;
    /// use stem_winding::prelude::*;
    ///
    /// let short_pitched = DistributedWinding::new(DistributedMinimalBuilder {
    ///     slots: 12.try_into().expect("not zero"),
    ///     pole_pairs: 1.try_into().expect("not zero"),
    ///     phases: 3.try_into().expect("not zero"),
    ///     layers: 2.try_into().expect("not zero"),
    ///     winding_table_constructor: WindingTableConstructor::Tingley,
    ///     coil_throw_reduction: 1,
    ///     zone_shift: 0,
    /// }).expect("valid winding topology");
    ///
    /// // Coil span is 5
    /// assert_eq!(
    ///     short_pitched.coil_at([0, 0].into()).expect("exists").throw(Some(short_pitched.slots())),
    ///     5
    /// );
    ///
    /// // Winding factor for the fundamental and the first non-zero superharmonic
    /// let ph = NonZeroU16::new(1).expect("not zero");
    /// assert_abs_diff_eq!(
    ///     short_pitched.winding_factor(ph, SpatialOrder::Electrical(1)),
    ///     0.9330,
    ///     epsilon = 1e-3
    /// );
    /// assert_abs_diff_eq!(
    ///     short_pitched.winding_factor(ph, SpatialOrder::Electrical(5)),
    ///     0.0669,
    ///     epsilon = 1e-3
    /// );
    /// ```
    pub fn coil_throw_reduction(&self) -> i32 {
        self.coil_throw_reduction
    }

    /// Returns the zone shift for a double-layer winding in slots. For a
    /// single-layer winding, this value is meaningless.
    ///
    /// As discussed in the documentation of [`WindingTable::shift_zones`], it
    /// is possible to manipulate the winding factor of a double-layer winding
    /// by moving positive-polarity zones in one layer and negative-polarity
    /// zones in the other layer. This shift can be either done completely or
    /// partially depending on the `zone_shift` value: If it is equal to the
    /// [`Winding::hole_number`], a full shift is performed, otherwise the shift
    /// is only partial.
    ///
    /// In contrast to the
    /// [`coil_throw_reduction`](DistributedWinding::coil_throw_reduction), this
    /// value does not influence the coil span.
    ///
    /// # Examples
    ///
    /// The following examples show a 12-slot, 2-pole topology with different
    /// `zone_shift` values.
    ///
    /// ## Shift of 0
    #[doc = ""]
    #[cfg_attr(
        feature = "doc-images",
        doc = "![12/2 distributed winding][table_distributed_winding_zone_shift_0]"
    )]
    #[cfg_attr(
        feature = "doc-images",
        embed_doc_image::embed_doc_image(
            "table_distributed_winding_zone_shift_0",
            "docs/img/table_distributed_winding_zone_shift_0.svg"
        )
    )]
    #[cfg_attr(
        not(feature = "doc-images"),
        doc = "**Doc images not enabled**. Compile docs with
    `cargo doc --features 'doc-images'` and Rust version >= 1.54."
    )]
    /// ```
    /// use std::num::NonZeroU16;
    /// use approxim::assert_abs_diff_eq;
    /// use stem_winding::prelude::*;
    ///
    /// let winding = DistributedWinding::new(DistributedMinimalBuilder {
    ///     slots: 12.try_into().expect("not zero"),
    ///     pole_pairs: 1.try_into().expect("not zero"),
    ///     phases: 3.try_into().expect("not zero"),
    ///     layers: 2.try_into().expect("not zero"),
    ///     winding_table_constructor: WindingTableConstructor::Tingley,
    ///     coil_throw_reduction: 0,
    ///     zone_shift: 0,
    /// }).expect("valid winding topology");
    ///
    /// // Coil span is 6 (slots / (2 * pole_pairs))
    /// assert_eq!(
    ///     winding.coil_at([0, 0].into()).expect("exists").throw(Some(winding.slots())),
    ///     6
    /// );
    ///
    /// // Winding factor for the fundamental and the first non-zero superharmonic
    /// let ph = NonZeroU16::new(1).expect("not zero");
    /// assert_abs_diff_eq!(
    ///     winding.winding_factor(ph, SpatialOrder::Electrical(1)),
    ///     0.9659,
    ///     epsilon = 1e-3
    /// );
    /// assert_abs_diff_eq!(
    ///     winding.winding_factor(ph, SpatialOrder::Electrical(5)),
    ///     0.2588,
    ///     epsilon = 1e-3
    /// );
    /// ```
    /// ## Shift of 1
    #[doc = ""]
    #[cfg_attr(
        feature = "doc-images",
        doc = "![Zone-shifted 12/2 distributed winding][table_distributed_winding_zone_shift_1]"
    )]
    #[cfg_attr(
        feature = "doc-images",
        embed_doc_image::embed_doc_image(
            "table_distributed_winding_zone_shift_1",
            "docs/img/table_distributed_winding_zone_shift_1.svg"
        )
    )]
    #[cfg_attr(
        not(feature = "doc-images"),
        doc = "**Doc images not enabled**. Compile docs with
    `cargo doc --features 'doc-images'` and Rust version >= 1.54."
    )]
    /// ```
    /// use std::num::NonZeroU16;
    /// use approxim::assert_abs_diff_eq;
    /// use stem_winding::prelude::*;
    ///
    /// let winding = DistributedWinding::new(DistributedMinimalBuilder {
    ///     slots: 12.try_into().expect("not zero"),
    ///     pole_pairs: 1.try_into().expect("not zero"),
    ///     phases: 3.try_into().expect("not zero"),
    ///     layers: 2.try_into().expect("not zero"),
    ///     winding_table_constructor: WindingTableConstructor::Tingley,
    ///     coil_throw_reduction: 0,
    ///     zone_shift: 1,
    /// }).expect("valid winding topology");
    ///
    /// // Coil span is still 6
    /// assert_eq!(
    ///     winding.coil_at([0, 0].into()).expect("exists").throw(Some(winding.slots())),
    ///     6
    /// );
    ///
    /// // Winding factor for the fundamental and the first non-zero superharmonic
    /// let ph = NonZeroU16::new(1).expect("not zero");
    /// assert_abs_diff_eq!(
    ///     winding.winding_factor(ph, SpatialOrder::Electrical(1)),
    ///     0.9330,
    ///     epsilon = 1e-3
    /// );
    /// assert_abs_diff_eq!(
    ///     winding.winding_factor(ph, SpatialOrder::Electrical(5)),
    ///     0.0669,
    ///     epsilon = 1e-3
    /// );
    /// ```
    /// ## Shift of 2
    ///
    /// This is now a 3-zone winding instead of the original 6-zone winding.
    #[doc = ""]
    #[cfg_attr(
        feature = "doc-images",
        doc = "![Zone-shifted 12/2 distributed winding][table_distributed_winding_zone_shift_2]"
    )]
    #[cfg_attr(
        feature = "doc-images",
        embed_doc_image::embed_doc_image(
            "table_distributed_winding_zone_shift_2",
            "docs/img/table_distributed_winding_zone_shift_2.svg"
        )
    )]
    #[cfg_attr(
        not(feature = "doc-images"),
        doc = "**Doc images not enabled**. Compile docs with
    `cargo doc --features 'doc-images'` and Rust version >= 1.54."
    )]
    /// ```
    /// use std::num::NonZeroU16;
    /// use approxim::assert_abs_diff_eq;
    /// use stem_winding::prelude::*;
    ///
    /// let winding = DistributedWinding::new(DistributedMinimalBuilder {
    ///     slots: 12.try_into().expect("not zero"),
    ///     pole_pairs: 1.try_into().expect("not zero"),
    ///     phases: 3.try_into().expect("not zero"),
    ///     layers: 2.try_into().expect("not zero"),
    ///     winding_table_constructor: WindingTableConstructor::Tingley,
    ///     coil_throw_reduction: 0,
    ///     zone_shift: 2,
    /// }).expect("valid winding topology");
    ///
    /// // Coil span is still 6
    /// assert_eq!(
    ///     winding.coil_at([0, 0].into()).expect("exists").throw(Some(winding.slots())),
    ///     6
    /// );
    ///
    /// // Winding factor for the fundamental and the first non-zero superharmonic
    /// let ph = NonZeroU16::new(1).expect("not zero");
    /// assert_abs_diff_eq!(
    ///     winding.winding_factor(ph, SpatialOrder::Electrical(1)),
    ///     0.8365,
    ///     epsilon = 1e-3
    /// );
    /// assert_abs_diff_eq!(
    ///     winding.winding_factor(ph, SpatialOrder::Electrical(5)),
    ///     0.2241,
    ///     epsilon = 1e-3
    /// );
    /// ```
    pub fn zone_shift(&self) -> u16 {
        self.zone_shift
    }

    /// Returns the [`Wire`] used by all coils of `self`.
    ///
    /// As stated in the struct documentation, all coils of a
    /// [`DistributedWinding`] use the same wire by definition.
    pub fn wire(&self) -> &dyn Wire {
        &*self.wire
    }

    /// Returns the [`WindingTableConstructor`] used to create the
    /// [`WindingTable`] returned by the
    /// [`winding_table`](Winding::winding_table) method.
    pub fn winding_table_constructor(&self) -> &WindingTableConstructor {
        &self.winding_table_constructor
    }

    /// Returns the coefficient used for the
    /// [`end_winding_leakage_inductance`](Winding::end_winding_leakage_inductance)
    /// calculation.
    ///
    /// [`DistributedWinding`] uses the
    /// [`end_winding_leakage_inductance_distributed`] function for implementing
    /// [`Winding::end_winding_leakage_inductance`]. This is the value passed
    /// into that function.
    pub fn end_winding_leakage_coefficient(&self) -> f64 {
        self.end_winding_leakage_coefficient
    }

    /// Returns the mean end-winding half-turn length, if one was provided.
    ///
    /// If this value is `Some`, [`DistributedWinding`] uses it for calculations
    /// such as [`Winding::resistance`] instead of calculating the length.
    /// Otherwise, the length is calculated on demand using
    /// [`end_winding_half_turn_length_straight`] for a linear core or
    /// [`end_winding_half_turn_length_circular_arc`] for a rotary core.
    /// Providing a value is useful when the length is known from
    /// measurements or more precise models.
    pub fn mean_end_winding_half_turn_length(&self) -> Option<Length> {
        self.mean_end_winding_half_turn_length
    }

    /// Sets the mean end-winding half-turn length.
    ///
    /// The value must be non-negative. If a negative length is provided, the
    /// method returns an error and leaves the existing value unchanged. Passing
    /// `None` clears the value, so the length is calculated on demand. See
    /// [`DistributedWinding::mean_end_winding_half_turn_length`] for details on
    /// how the value is used.
    pub fn set_mean_end_winding_half_turn_length(
        &mut self,
        mean_end_winding_half_turn_length: Option<Length>,
    ) -> Result<(), Comparison<Length>> {
        if let Some(mean_end_winding_half_turn_length) = mean_end_winding_half_turn_length {
            let zero_length = Length::new::<meter>(0.0);
            compare_variables!(mean_end_winding_half_turn_length >= zero_length)?;
        }
        self.mean_end_winding_half_turn_length = mean_end_winding_half_turn_length;
        Ok(())
    }

    /// Returns true if the winding is built from concentric coils.
    ///
    /// If the coils are not concentric, all coils of a coil group has the same
    /// [`throw`](crate::coils::CoilExt::throw) as shown in the image below:
    #[doc = ""]
    #[cfg_attr(
        feature = "doc-images",
        doc = "![12/2 distributed winding w/o concentric coils][coils_distributed_winding_concentric_false]"
    )]
    #[cfg_attr(
        feature = "doc-images",
        embed_doc_image::embed_doc_image(
            "coils_distributed_winding_concentric_false",
            "docs/img/coils_distributed_winding_concentric_false.svg"
        )
    )]
    #[cfg_attr(
        not(feature = "doc-images"),
        doc = "**Doc images not enabled**. Compile docs with
    `cargo doc --features 'doc-images'` and Rust version >= 1.54."
    )]
    ///
    /// Concentric coils are arranged as concentric rings with increasing coil
    /// throw as shown below. This can be advantageous for manufacturing.
    #[doc = ""]
    #[cfg_attr(
        feature = "doc-images",
        doc = "![12/2 distributed winding w/ concentric coils][coils_distributed_winding_concentric_true]"
    )]
    #[cfg_attr(
        feature = "doc-images",
        embed_doc_image::embed_doc_image(
            "coils_distributed_winding_concentric_true",
            "docs/img/coils_distributed_winding_concentric_true.svg"
        )
    )]
    #[cfg_attr(
        not(feature = "doc-images"),
        doc = "**Doc images not enabled**. Compile docs with
    `cargo doc --features 'doc-images'` and Rust version >= 1.54."
    )]
    ///
    /// Because the winding table is not influenced, this parameter does not
    /// influence the qualitative winding analysis, which is why this
    /// parameter is missing from the [`DistributedMinimalBuilder`]. However, it
    /// can (slightly) change the quantitative parameters such as the
    /// resistance.
    pub fn concentric_coils(&self) -> bool {
        return self.concentric_coils;
    }

    /// Sets the [`DistributedWinding::concentric_coils`] parameter.
    ///
    /// See the documentation of [`DistributedWinding::concentric_coils`] for
    /// details. If this parameter is changed, the coils of `self` are rebuild
    /// to adhere to the new topology.
    pub fn set_concentric_coils(&mut self, concentric_coils: bool) {
        // Do nothing if the layout already equals the desired layout.
        if concentric_coils == self.concentric_coils() {
            return ();
        }

        self.concentric_coils = concentric_coils;

        // Get the zone plan
        let winding_table = self.winding_table(false);

        // Clear all coils
        self.coils.clear();

        // Rebuild the coils with the zone plan
        self.create_coils(&winding_table).expect(
            "since self already had a legal coil configuration, rebuilding it must succeed.",
        );
    }

    fn create_coils(&mut self, winding_table: &WindingTable) -> Result<(), Error> {
        // This algorithm creates the coils of self by first grouping the zones
        // into pairs of coil groups and then creating coils going from one pair
        // to the other.
        let return_layer = if self.layers().get() == 1 { 0 } else { 1 };

        // The three phases are handled independently from each other, since they don't
        // interact.
        for phase in 1..(self.phases().get() as i32 + 1) {
            let start_slot = largest_span_slot(winding_table, phase);
            for slot in start_slot..(self.slots().get() + start_slot) {
                let slot = slot.rem_euclid(self.slots().get());

                /*
                It is sufficient to search on layer 0, since all coils in a
                double-layer winding go from layer 0 to layer 1 (return_layer).
                 */
                let zone = Zone::new(slot, 0);

                // Skip all zones which don't belong to the current phase
                if winding_table.get_cyclic(zone).abs() != phase {
                    continue;
                }

                // Identify the coil group pair
                if let Some(seed_group) = AvailableSlotsForCoilGroup::new(
                    winding_table,
                    &self.coils,
                    zone,
                    self.slots().get(),
                ) {
                    if let Some((partner_group, found_partner_while_ascending)) = seed_group
                        .find_partner(winding_table, &self.coils, self.slots().get(), return_layer)
                    {
                        let (pos, neg, positive_slot_direction) = if seed_group.phase > 0 {
                            (seed_group, partner_group, found_partner_while_ascending)
                        } else {
                            (partner_group, seed_group, !found_partner_while_ascending)
                        };
                        if self.concentric_coils {
                            // Connect pos.start to neg.stop
                            for (pos_slot, neg_slot) in (pos.start..(pos.start + pos.len + 1))
                                .into_iter()
                                .zip((neg.start..(neg.start + neg.len + 1)).into_iter().rev())
                            {
                                let positive_zone =
                                    Zone::new(pos_slot.rem_euclid(self.slots().get()), pos.layer);
                                let negative_zone =
                                    Zone::new(neg_slot.rem_euclid(self.slots().get()), neg.layer);
                                let coil = FullCoil::new(
                                    positive_zone,
                                    negative_zone,
                                    positive_slot_direction,
                                    self.turns_per_coil,
                                    NonZeroU16::new(pos.phase as u16).expect("cannot be zero"),
                                    clone_box(&*self.wire),
                                )?;
                                self.coils.insert(Coil::Full(coil))?;
                            }
                        } else {
                            // Connect pos.start to neg.start and pos.stop to neg.stop
                            for (pos_slot, neg_slot) in (pos.start..(pos.start + pos.len + 1))
                                .into_iter()
                                .zip((neg.start..(neg.start + neg.len + 1)).into_iter())
                            {
                                let positive_zone =
                                    Zone::new(pos_slot.rem_euclid(self.slots().get()), pos.layer);
                                let negative_zone =
                                    Zone::new(neg_slot.rem_euclid(self.slots().get()), neg.layer);
                                let coil = FullCoil::new(
                                    positive_zone,
                                    negative_zone,
                                    positive_slot_direction,
                                    self.turns_per_coil,
                                    NonZeroU16::new(pos.phase as u16).expect("cannot be zero"),
                                    clone_box(&*self.wire),
                                )?;
                                self.coils.insert(Coil::Full(coil))?;
                            }
                        }
                    }
                }
            }
        }

        // Check if all zones are filled.
        for (zone, _) in winding_table.iter_slot_major() {
            // Check if the zone is already occupied
            if !self.coils.occupied(zone) {
                return Err(
                    crate::error::WindingTableConstructionError::EmptyZone(Some(zone)).into(),
                );
            }
        }
        return Ok(());
    }
}

/**
Finds the first slot where the coil span is maximized.

The coil builder algorithm of the [`DistributedWinding`] is very sensitive to
the starting location. As an example, let's consider a 18 slot / 4 pole pairs
single layer winding with the following zone plan:
```text
1 2 -1 3 -2 -3 2 3 -2 1 -3 -1 3 1 -3 2 -1 -2
```
Starting the coil builder algorithm at the first slot results in the following
coil configuration for phase 2:
```text
──┐       ┌────┐    ┌────────────────┐     ┌
1 2 -1 3 -2 -3 2 3 -2 1 -3 -1 3 1 -3 2 -1 -2
──┘       └────┘    └────────────────┘     └
```
The very long third coil is solely a result of the starting location. If we
instead start at the third positive zone, the coil configuration looks like this:
  ```text
  ┌───────┐    ┌────┐                ┌─────┐
1 2 -1 3 -2 -3 2 3 -2 1 -3 -1 3 1 -3 2 -1 -2
  └───────┘    └────┘                └─────┘
```
The overall end winding length of this configuration is much shorter. To find
the optimal configuration with a minimum overall end winding length, a heuristic
approach is used: We search for the hypothetical coil with the longest span and
start the coil creater algorithm at the positive zone of this coil.
 */
fn largest_span_slot(winding_table: &WindingTable, phase: i32) -> u16 {
    let mut longest_span = 0;
    let mut span_start_slot = 0;
    let mut positive_slot: u16 = 0;
    let mut last_phase: i32 = 0;

    /*
    In order to find coils which go from the end to the start of the zone plan,
    the algorithm scans the zone plan two times.
     */
    for slot in 0..(2 * winding_table.slots().get()) {
        /*
        We only need to check layer 0, since in case of a double-layer winding layer 1
        is merely a shifted and mirrored version of layer 0.
         */
        let current_phase = winding_table.get_cyclic(Zone::new(slot, 0)).clone();
        if current_phase.abs() == phase {
            if current_phase != last_phase {
                let current_span = slot - span_start_slot;
                if current_span > longest_span {
                    longest_span = current_span;
                    if current_phase > 0 {
                        positive_slot = slot
                    } else {
                        positive_slot = span_start_slot;
                    }
                }
                last_phase = current_phase;
            }
            span_start_slot = slot;
        }
    }
    return positive_slot;
}

/**
A coil group starts in slot `start` and extends for `len` slots.
This extension is wrapping: If e.g. the total number of slots is 12, start is 11 and len is 2,
the coil group is located in the slots 11 and 0.
*/
#[derive(Clone, Debug, PartialEq)]
struct AvailableSlotsForCoilGroup {
    start: u16,
    stop: u16,
    layer: u16,
    len: u16,
    phase: i32,
}

impl AvailableSlotsForCoilGroup {
    /**
    Get the coil group zones of the coil at `zone`.
     */
    fn new(
        winding_table: &WindingTable,
        coils: &Coils,
        zone: Zone,
        slots: u16,
    ) -> Option<AvailableSlotsForCoilGroup> {
        // If the "seed" is already occupied, abort.
        if coils.occupied(zone) {
            return None;
        }

        let phase = winding_table.get_cyclic(zone);

        // Find the start
        let mut start = zone.slot;
        for slot in (zone.slot..(zone.slot + slots)).rev() {
            let new_zone = Zone::new(slot.rem_euclid(slots), zone.layer);
            if coils.occupied(new_zone) || winding_table.get_cyclic(new_zone) != phase {
                break;
            } else {
                start = new_zone.slot;
            }
        }

        // Find the length
        let mut stop = zone.slot;
        for slot in (zone.slot + 1)..(zone.slot + slots) {
            let new_zone = Zone::new(slot.rem_euclid(slots), zone.layer);
            if coils.occupied(new_zone) || winding_table.get_cyclic(new_zone) != phase {
                break;
            } else {
                stop = new_zone.slot;
            }
        }
        let len = (stop + slots - start).rem_euclid(slots);

        return Some(AvailableSlotsForCoilGroup {
            start,
            stop,
            len,
            phase: *phase,
            layer: zone.layer,
        });
    }

    /**
    Try to find a partner for `self` on the specified `layer`. If no partner can be identified, return None.
    If a partner could be identified, also return the info whether the partner has been found while ascending
     */
    fn find_partner(
        &self,
        winding_table: &WindingTable,
        coils: &Coils,
        slots: u16,
        search_layer: u16,
    ) -> Option<(Self, bool)> {
        for delta in 0..slots {
            let slot_asc = (self.stop + delta).rem_euclid(slots);
            let slot_des = (self.start + slots - delta).rem_euclid(slots);
            for (is_des, search_slot) in [slot_asc, slot_des].into_iter().enumerate() {
                let search_zone = Zone::new(search_slot, search_layer);
                if *winding_table.get_cyclic(search_zone) == -self.phase {
                    if let Some(potential_partner) =
                        Self::new(winding_table, coils, search_zone, slots)
                    {
                        if potential_partner.len >= self.len {
                            return Some((potential_partner, is_des == 0));
                        } else {
                            return None;
                        }
                    }
                }
            }
        }
        return None;
    }
}

impl Default for DistributedWinding {
    fn default() -> Self {
        DistributedBuilder {
            slots: NonZeroU16::new(6).expect("not zero"),
            pole_pairs: NonZeroU16::MIN,
            phases: NonZeroU16::new(3).expect("not zero"),
            layers: NonZeroU16::MIN,
            coil_throw_reduction: 0,
            zone_shift: 0,
            turns_per_coil: NonZeroUsize::MIN,
            parallel_paths: NonZeroU16::MIN,
            connection: Connection::Star,
            end_winding_leakage_coefficient: 0.0,
            wire: Box::new(SffWire::default()),
            winding_table_constructor: WindingTableConstructor::Tingley,
            concentric_coils: false,
            mean_end_winding_half_turn_length: None,
        }
        .try_into()
        .expect("valid inputs")
    }
}

#[cfg_attr(feature = "serde", typetag::serde)]
impl Winding for DistributedWinding {
    fn phases(&self) -> NonZeroU16 {
        return self.phases;
    }

    fn slots(&self) -> NonZeroU16 {
        return self.slots;
    }

    fn pole_pairs(&self) -> NonZeroU16 {
        return self.pole_pairs;
    }

    fn layers(&self) -> NonZeroU16 {
        return self.layers;
    }

    fn base_winding_count(&self) -> NonZeroU16 {
        return base_winding_count_symmetric(
            self.slots(),
            self.pole_pairs(),
            self.phases(),
            self.layers(),
        );
    }

    fn turns_at(&self, _zone: Zone) -> usize {
        return self.turns_per_coil.get();
    }

    fn coil_layout(&self) -> CoilLayout {
        if self.layers().get() == 1 {
            return CoilLayout::Single;
        } else {
            return CoilLayout::DoubleVertical;
        }
    }

    fn series_turns_per_phase(&self, phase: NonZeroU16) -> Ratio<usize> {
        return Ratio::new_raw(
            usize::from(self.layers().get() * self.slots().get()) * self.turns_per_coil.get()
                / usize::from(2 * self.phases().get() * self.parallel_paths(phase).get()),
            1,
        );
    }

    fn parallel_paths(&self, _phase: NonZeroU16) -> NonZeroU16 {
        return self.parallel_paths;
    }

    fn connection(&self) -> Connection {
        return self.connection;
    }

    fn coil_at(&self, zone: Zone) -> Option<&Coil> {
        return self.coils.get(zone);
    }

    fn as_dyn(&self) -> &dyn Winding {
        self
    }

    #[cfg(feature = "stem_core")]
    fn is_symmetric(&self, _core: CoreRef<'_>) -> bool {
        return true;
    }

    #[cfg(feature = "stem_core")]
    fn end_winding_leakage_inductance(
        &self,
        core: CoreRef<'_>,
        _phase: NonZeroU16,
        spatial_order: SpatialOrder,
        conditions: &[DynQuantity<f64>],
    ) -> Inductance {
        let _ = spatial_order;
        let _ = conditions;
        end_winding_leakage_inductance_distributed(self, core, self.end_winding_leakage_coefficient)
    }

    #[cfg(feature = "stem_core")]
    fn end_winding_half_turn_length(&self, core: CoreRef<'_>, zone: Zone) -> Length {
        self.mean_end_winding_half_turn_length
            .unwrap_or_else(|| match core {
                CoreRef::Lin(lin_core) => {
                    end_winding_half_turn_length_straight(self, lin_core, zone)
                }
                CoreRef::Rot(rot_core) => {
                    end_winding_half_turn_length_circular_arc(self, rot_core, zone)
                }
            })
    }
}

// =============================================================================
// Builders

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct DistributedBuilder {
    pub slots: NonZeroU16,
    pub pole_pairs: NonZeroU16,
    pub phases: NonZeroU16,
    pub layers: NonZeroU16,
    pub coil_throw_reduction: i32,
    pub zone_shift: u16,
    pub winding_table_constructor: WindingTableConstructor,
    pub turns_per_coil: NonZeroUsize,
    pub parallel_paths: NonZeroU16,
    pub connection: Connection,
    pub end_winding_leakage_coefficient: f64,
    pub wire: Box<dyn Wire>,
    pub concentric_coils: bool,
    #[cfg_attr(feature = "serde", serde(default))]
    pub mean_end_winding_half_turn_length: Option<Length>,
}

impl TryFrom<DistributedBuilder> for DistributedWinding {
    type Error = Error;

    fn try_from(builder: DistributedBuilder) -> Result<Self, Self::Error> {
        let layers = builder.layers.get();
        let slots = builder.slots.get();
        let pole_pairs = builder.pole_pairs.get();

        compare_variables!(layers < 3)?;
        compare_variables!(0.0 <= builder.end_winding_leakage_coefficient)?;

        // Check if the number of parallel paths is valid
        let max_par_paths = super::max_parallel_paths_symmetric(
            builder.slots,
            builder.pole_pairs,
            builder.phases,
            builder.layers,
        )
        .get();
        if max_par_paths % builder.parallel_paths.get() != 0 {
            return Err(Error::InvalidNumberParallelPaths);
        }

        // Calculate the basic winding parameters
        let t = base_winding_count_symmetric(
            builder.slots,
            builder.pole_pairs,
            builder.phases,
            builder.layers,
        );
        let slots_basic = slots / t;
        let pole_pairs_basic = pole_pairs / t;

        let span = (slots as f64 / (2.0 * pole_pairs as f64)).floor() as i32
            - builder.coil_throw_reduction;

        // Create the zone plan by method
        let mut winding_table = WindingTable::from_constructor(
            &builder.winding_table_constructor,
            NonZeroU16::new(slots_basic).expect("not zero"),
            builder.layers,
            NonZeroU16::new(pole_pairs_basic).expect("not zero"),
            builder.phases,
            span,
        )?;

        // Modify the zone span accordingly
        winding_table.shift_zones(builder.zone_shift as i32);

        let num_zones = (builder.slots.get() * builder.layers.get()).into();
        let num_coils = num_zones / 2;
        let mut winding = DistributedWinding {
            slots: builder.slots,
            pole_pairs: builder.pole_pairs,
            phases: builder.phases,
            layers: builder.layers,
            coil_throw_reduction: builder.coil_throw_reduction,
            zone_shift: builder.zone_shift,
            turns_per_coil: builder.turns_per_coil,
            parallel_paths: builder.parallel_paths,
            connection: builder.connection,
            end_winding_leakage_coefficient: builder.end_winding_leakage_coefficient,
            wire: builder.wire,
            coils: Coils::with_capacity(num_zones, num_coils),
            winding_table_constructor: builder.winding_table_constructor,
            concentric_coils: builder.concentric_coils,
            mean_end_winding_half_turn_length: None,
        };
        winding.set_mean_end_winding_half_turn_length(builder.mean_end_winding_half_turn_length)?;
        winding.create_coils(&winding_table)?;

        if winding.equal_winding_factors() {
            return Ok(winding);
        } else {
            return Err(WindingTableConstructionError::NotSymmetric.into());
        }
    }
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct DistributedDoubleZoneSpanBuilder {
    pub slots: NonZeroU16,
    pub pole_pairs: NonZeroU16,
    pub phases: NonZeroU16,
    pub coil_throw_reduction: i32,
    pub turns_per_coil: NonZeroUsize,
    pub parallel_paths: NonZeroU16,
    pub connection: Connection,
    pub end_winding_leakage_coefficient: f64,
    pub wire: Box<dyn Wire>,
    pub winding_table_constructor: WindingTableConstructor,
    pub concentric_coils: bool,
    #[cfg_attr(feature = "serde", serde(default))]
    pub mean_end_winding_half_turn_length: Option<Length>,
}

impl TryFrom<DistributedDoubleZoneSpanBuilder> for DistributedWinding {
    type Error = Error;

    fn try_from(builder: DistributedDoubleZoneSpanBuilder) -> Result<Self, Self::Error> {
        // Check if an integer slot winding can be constructed from the input
        let hn = hole_number(builder.slots, builder.pole_pairs, builder.phases);
        let g = hn.trunc().numer().clone();
        let n = hn.denom().clone();
        let zone_shift = if n == 1 {
            g
        } else {
            return Err(Error::NeedsIntegerSlot);
        };

        DistributedBuilder {
            slots: builder.slots,
            pole_pairs: builder.pole_pairs,
            phases: builder.phases,
            layers: NonZeroU16::new(2).expect("not zero"),
            coil_throw_reduction: builder.coil_throw_reduction,
            zone_shift,
            winding_table_constructor: builder.winding_table_constructor,
            turns_per_coil: builder.turns_per_coil,
            parallel_paths: builder.parallel_paths,
            connection: builder.connection,
            end_winding_leakage_coefficient: builder.end_winding_leakage_coefficient,
            wire: builder.wire,
            concentric_coils: builder.concentric_coils,
            mean_end_winding_half_turn_length: builder.mean_end_winding_half_turn_length,
        }
        .try_into()
    }
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct DistributedMinimalBuilder {
    pub slots: NonZeroU16,
    pub pole_pairs: NonZeroU16,
    pub phases: NonZeroU16,
    pub layers: NonZeroU16,
    /// Coil span reduction in slots ("Wickelschrittverkürzung aufgrund der
    /// Sehnung")
    pub coil_throw_reduction: i32,
    pub zone_shift: u16,
    pub winding_table_constructor: WindingTableConstructor,
}

impl TryFrom<DistributedMinimalBuilder> for DistributedWinding {
    type Error = Error;

    fn try_from(builder: DistributedMinimalBuilder) -> Result<Self, Self::Error> {
        return DistributedBuilder {
            slots: builder.slots,
            pole_pairs: builder.pole_pairs,
            phases: builder.phases,
            layers: builder.layers,
            coil_throw_reduction: builder.coil_throw_reduction,
            zone_shift: builder.zone_shift,
            winding_table_constructor: builder.winding_table_constructor,
            turns_per_coil: NonZeroUsize::MIN,
            parallel_paths: NonZeroU16::MIN,
            connection: Connection::Star,
            end_winding_leakage_coefficient: 0.0,
            wire: Box::new(SffWire::default()),
            concentric_coils: false,
            mean_end_winding_half_turn_length: None,
        }
        .try_into();
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for DistributedWinding {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(deserialize_untagged_verbose_error::DeserializeUntaggedVerboseError)]
        enum DistributedEnum {
            DistributedBuilder(DistributedBuilder),
            DistributedDoubleZoneSpanBuilder(DistributedDoubleZoneSpanBuilder),
            DistributedMinimalBuilder(DistributedMinimalBuilder),
        }
        let w = DistributedEnum::deserialize(deserializer)?;
        match w {
            DistributedEnum::DistributedBuilder(w) => {
                w.try_into().map_err(serde::de::Error::custom)
            }
            DistributedEnum::DistributedDoubleZoneSpanBuilder(w) => {
                w.try_into().map_err(serde::de::Error::custom)
            }
            DistributedEnum::DistributedMinimalBuilder(w) => {
                w.try_into().map_err(serde::de::Error::custom)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_coil_group() {
        let winding_table = WindingTable::from_constructor(
            &WindingTableConstructor::Tingley,
            18.try_into().unwrap(),
            1.try_into().unwrap(),
            1.try_into().unwrap(),
            3.try_into().unwrap(),
            9,
        )
        .unwrap();
        let coils = Coils::new();

        let cg =
            AvailableSlotsForCoilGroup::new(&winding_table, &coils, Zone::new(0, 0), 18).unwrap();
        assert_eq!(cg.start, 0);
        assert_eq!(cg.stop, 2);
        assert_eq!(cg.len, 2);
        assert_eq!(cg.phase, 1);

        let cg =
            AvailableSlotsForCoilGroup::new(&winding_table, &coils, Zone::new(1, 0), 18).unwrap();
        assert_eq!(cg.start, 0);
        assert_eq!(cg.stop, 2);
        assert_eq!(cg.len, 2);
        assert_eq!(cg.phase, 1);
    }

    #[test]
    fn test_find_partners() {
        let winding_table = WindingTable::from_constructor(
            &WindingTableConstructor::Tingley,
            18.try_into().unwrap(),
            1.try_into().unwrap(),
            1.try_into().unwrap(),
            3.try_into().unwrap(),
            9,
        )
        .unwrap();
        let coils = Coils::new();

        let cg =
            AvailableSlotsForCoilGroup::new(&winding_table, &coils, Zone::new(0, 0), 18).unwrap();
        let (partner, found_while_ascending) =
            cg.find_partner(&winding_table, &coils, 18, 0).unwrap();
        assert!(found_while_ascending);
        assert_eq!(partner.start, 9);
        assert_eq!(partner.stop, 11);
        assert_eq!(partner.len, 2);
        assert_eq!(partner.phase, -1);
    }
}
