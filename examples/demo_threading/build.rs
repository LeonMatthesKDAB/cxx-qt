// SPDX-FileCopyrightText: 2021 Klarälvdalens Datakonsult AB, a KDAB Group company <info@kdab.com>
// SPDX-FileContributor: Andrew Hayzen <andrew.hayzen@kdab.com>
//
// SPDX-License-Identifier: MIT OR Apache-2.0
use cc::Build;
use cxx_qt_build::CxxQtBuilder;

fn build_cc_file(cc: &mut Build, name: &str) {
    cc.file(name);
    println!("cargo:rerun-if-changed={}", name);
}

fn main() {
    CxxQtBuilder::new()
        .qt_modules(&["Qml", "Network"])
        .file("src/lib.rs")
        .qrc("qml/qml.qrc")
        .qrc("qml/compat/compat_qt5.qrc")
        .qobject_header("cpp/helpers/energyusageproxymodel.h")
        .qobject_header("cpp/helpers/sensor.h")
        .cc_builder(|cc| {
            build_cc_file(cc, "cpp/main.cpp");
            build_cc_file(cc, "cpp/helpers/energyusageproxymodel.cpp");
            build_cc_file(cc, "cpp/helpers/sensor.cpp");
            cc.file("cpp/main.cpp");
            println!("cargo:rerun-if-changed=cpp/main.cpp");
        })
        .build();
}
