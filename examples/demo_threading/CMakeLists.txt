# SPDX-FileCopyrightText: 2021 Klarälvdalens Datakonsult AB, a KDAB Group company <info@kdab.com>
# SPDX-FileContributor: Andrew Hayzen <andrew.hayzen@kdab.com>
#
# SPDX-License-Identifier: MIT OR Apache-2.0

# TODO: figure out what the actual hard minimum is
cmake_minimum_required(VERSION 3.16)

project(demo_threading)
set(APP_NAME ${PROJECT_NAME})

# TODO: Add a helper function to our CMake module which automatically
# handles some of this boilerplate for a "typical" Qt application
set(CMAKE_AUTOMOC ON)
set(CMAKE_AUTORCC ON)
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

find_package(QT NAMES Qt6 Qt5 COMPONENTS Core Gui Qml QuickControls2 QmlImportScanner QuickTest Test REQUIRED)
find_package(Qt${QT_VERSION_MAJOR} COMPONENTS Core Gui Qml QuickControls2 QmlImportScanner QuickTest Test REQUIRED)

list(APPEND CMAKE_MODULE_PATH "${CMAKE_CURRENT_LIST_DIR}/../../cmake")
include(CxxQt)

set(
    CPP_SOURCES
    ${CMAKE_CURRENT_SOURCE_DIR}/src/main.cpp
    ${CMAKE_CURRENT_SOURCE_DIR}/src/helpers/energyusageproxymodel.h
    ${CMAKE_CURRENT_SOURCE_DIR}/src/helpers/energyusageproxymodel.cpp
    ${CMAKE_CURRENT_SOURCE_DIR}/src/helpers/sensor.h
    ${CMAKE_CURRENT_SOURCE_DIR}/src/helpers/sensor.cpp
)

set(
    RESOURCES
    ${CMAKE_CURRENT_SOURCE_DIR}/src/ui/qml.qrc
)

# Generate the C++ code
file(STRINGS "$ENV{OECORE_TARGET_SYSROOT}/usr/include/demo-threading/target/cxx-qt-gen/cpp_sources.txt" GEN_SOURCES)
list(TRANSFORM GEN_SOURCES PREPEND "$ENV{OECORE_TARGET_SYSROOT}/usr/include/demo-threading/target/")

# Define our executable with our QML resource files
add_executable(${APP_NAME} "${CPP_SOURCES}" "${GEN_SOURCES}" "${RESOURCES}")

# Include generated sources
target_include_directories(${APP_NAME} PUBLIC "$ENV{OECORE_TARGET_SYSROOT}/usr/include/demo-threading/target")
# Our cxx_qt and cxx headers are in this folder and need to be included
target_include_directories(${APP_NAME} PUBLIC "$ENV{OECORE_TARGET_SYSROOT}/usr/include/demo-threading/target/cxx-qt-gen/statics")

# Link to generated rust library
set(CMAKE_THREAD_PREFER_PTHREAD TRUE)
find_package(Threads REQUIRED)
target_link_libraries(${APP_NAME} PRIVATE rust Threads::Threads ${CMAKE_DL_LIBS})


target_link_libraries(${APP_NAME} PRIVATE
    Qt${QT_VERSION_MAJOR}::Core
    Qt${QT_VERSION_MAJOR}::Gui
    Qt${QT_VERSION_MAJOR}::Qml
    Qt${QT_VERSION_MAJOR}::QuickControls2
    Qt${QT_VERSION_MAJOR}::Test
)
# qt_import_qml_plugins(${APP_NAME})
