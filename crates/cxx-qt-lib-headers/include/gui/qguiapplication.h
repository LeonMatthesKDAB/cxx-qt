// clang-format off
// SPDX-FileCopyrightText: 2023 Klarälvdalens Datakonsult AB, a KDAB Group company <info@kdab.com>
// clang-format on
// SPDX-FileContributor: Andrew Hayzen <andrew.hayzen@kdab.com>
//
// SPDX-License-Identifier: MIT OR Apache-2.0
#pragma once

#ifdef CXX_QT_GUI_FEATURE
#include <cstdint>
#include <memory>

#include <QtGui/QGuiApplication>

#include "rust/cxx.h"

namespace rust {
namespace cxxqtlib1 {

::std::int32_t
qguiapplicationExec(QGuiApplication& app);
::std::unique_ptr<QGuiApplication>
qguiapplicationNew(::rust::Vec<::rust::String> args);

}
}

#endif
