use crate::spec::{base, PanicStrategy, Target, TargetMetadata};

pub(crate) fn target() -> Target {
    let mut base = base::edos::opts();
    base.cpu = "x86-64".into();
    base.panic_strategy = PanicStrategy::Abort;
    base.features = "-avx,-avx2".into();

    Target {
        llvm_target: "x86_64-unknown-none".into(),
        pointer_width: 64,
        data_layout:
            "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128".into(),
        arch: "x86_64".into(),
        options: base,
        metadata: TargetMetadata {
            description: Some("x86-64 edos".into()),
            tier: None,
            host_tools: None,
            std: Some(true),

        },
    }
}
