use crate::spec::{
    Cc, LinkerFlavor, Lld, PanicStrategy, RelocModel, RelroLevel, StackProbeType, TargetOptions,
};

pub(crate) fn opts() -> TargetOptions {
    TargetOptions {
        os: "edos".into(),
        linker: Some("rust-lld".into()),
        linker_flavor: LinkerFlavor::Gnu(Cc::No, Lld::Yes),
        tls_model: crate::spec::TlsModel::Emulated,
        stack_probes: StackProbeType::Inline,
        relocation_model: RelocModel::Pic,
        position_independent_executables: true,
        static_position_independent_executables: true,
        no_default_libraries: true,
        pre_link_args: Default::default(),
        late_link_args: Default::default(),
        has_thread_local: false,
        panic_strategy: PanicStrategy::Abort,
        crt_static_default: false,
        crt_static_respected: false,
        requires_uwtable: false,
        eh_frame_header: false,
        executables: true,
        relro_level: RelroLevel::Off,
        ..Default::default()
    }
}
