//! Coverage for `Kernel`'s debug-time Drop assert that fires when the
//! consumer is still alive at kernel-drop time.
//!
//! The assert is `debug_assert_eq!(Arc::strong_count(&control_plane), 1, ...)`
//! in [`Kernel::drop`]. It exists because the kernel unconditionally frees
//! its deferred-deletion queue and backing memory on drop; if a consumer is
//! still traversing those, the result is undefined behavior. These tests
//! pin the contract so that removing or weakening the assert in `src/`
//! becomes visible in the test suite.

mod common;

use synaptic_kernel::epoch_consumer::EpochConsumer;
use synaptic_kernel::kernel::Kernel;
use synaptic_kernel::kernel_config::KernelConfig;

const NODE_META: usize = 4;
const NODE_ATTR: usize = 4;
const SYNAPSE_META: usize = 4;
const SYNAPSE_ATTR: usize = 4;

type TestKernel = Kernel<1, 1, 1>;
type TestConsumer = EpochConsumer<1, 1, 1>;

fn config() -> KernelConfig<1, 1, 1> {
    common::kernel_config_1_1(4, 4, NODE_META, NODE_ATTR, SYNAPSE_META, SYNAPSE_ATTR)
}

/// The canonical pattern: declare the consumer AFTER the kernel so it drops
/// first. Tuple bindings drop in reverse declaration order — the consumer's
/// `Arc<ControlPlane>` clone is released, refcount returns to 1, and the
/// kernel's Drop assert passes.
#[test]
fn consumer_declared_after_kernel_drops_cleanly() {
    let kernel = TestKernel::new(config());
    let consumer = TestConsumer::new(kernel.get_control_plane());

    // Use both to prove they're not optimized out.
    let _ = kernel.node_capacity();
    drop(consumer);
    drop(kernel);
}

/// Equivalent shape using explicit `drop(consumer)` before scope end. Some
/// readers find this more obvious than relying on declaration order.
#[test]
fn explicit_drop_consumer_before_kernel_passes() {
    let kernel = TestKernel::new(config());
    let consumer = TestConsumer::new(kernel.get_control_plane());

    drop(consumer);
    // kernel drops here at end of scope, refcount == 1.
    let _ = kernel.node_capacity();
}

/// Negative case: the consumer's `Arc<ControlPlane>` clone outlives the
/// kernel. In debug builds, [`Kernel::drop`] panics via debug_assert_eq.
///
/// This test is `#[cfg(debug_assertions)]` because the assert is compiled
/// out of release builds — the contract is documented but unenforced
/// in release.
#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "Consumer must be quiesced")]
fn kernel_drop_panics_if_arc_clone_outlives_it() {
    // Hold a clone of Arc<ControlPlane> beyond the kernel's drop. The
    // kernel's Drop assert sees strong_count == 2.
    let _cp_clone;
    {
        let kernel = TestKernel::new(config());
        _cp_clone = kernel.get_control_plane();
        // kernel drops here at end of inner scope; _cp_clone is still alive,
        // so refcount is 2 and the debug_assert_eq fires.
    }
}

/// The same negative case, expressed with a live `EpochConsumer` instead of
/// a bare `Arc<ControlPlane>`. The consumer holds the Arc clone internally,
/// so the assertion fires for the same reason.
#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "Consumer must be quiesced")]
fn kernel_drop_panics_if_consumer_outlives_it() {
    let _consumer;
    {
        let kernel = TestKernel::new(config());
        _consumer = TestConsumer::new(kernel.get_control_plane());
        // kernel drops at the end of this inner scope; _consumer (declared
        // outside) keeps the Arc<ControlPlane> alive, so strong_count is 2
        // at drop time and the debug_assert_eq fires.
    }
}
