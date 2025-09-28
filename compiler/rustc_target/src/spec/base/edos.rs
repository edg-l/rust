use crate::spec::{Cc, LinkerFlavor, Lld, PanicStrategy, RelocModel, StackProbeType, TargetOptions};

pub(crate) fn opts() -> TargetOptions {
    TargetOptions {
        os: "edos".into(),
        linker: Some("rust-lld".into()),
        linker_flavor: LinkerFlavor::Gnu(Cc::No, Lld::Yes),
        tls_model: crate::spec::TlsModel::Emulated,
        stack_probes: StackProbeType::Inline,
        relocation_model: RelocModel::Pic,
        static_position_independent_executables: true,
        no_default_libraries: true,
        pre_link_args: Default::default(),
        late_link_args: Default::default(),
        crt_static_default: false,
        crt_static_respected: false,
        panic_strategy: PanicStrategy::Abort,
        ..Default::default()
    }
}
