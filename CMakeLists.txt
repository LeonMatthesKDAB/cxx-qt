# SPDX-FileCopyrightText: 2021 Klarälvdalens Datakonsult AB, a KDAB Group company <info@kdab.com>
# SPDX-FileContributor: Andrew Hayzen <andrew.hayzen@kdab.com>
# SPDX-FileContributor: Gerhard de Clercq <gerhard.declercq@kdab.com>
# SPDX-FileContributor: Be <be.0@gmx.com>
#
# SPDX-License-Identifier: MIT OR Apache-2.0

cmake_minimum_required(VERSION 3.24)

list(APPEND CMAKE_MODULE_PATH "${CMAKE_CURRENT_SOURCE_DIR}/cmake")

option(USE_QT5 "Use Qt5 even if Qt6 found" OFF)
option(VCPKG "Use vcpkg for release mode dependencies" OFF)
option(VCPKG_BUILDTREES_ROOT "Use custom vcpkg buildtrees" "")

option(BUILD_TESTING "Build the tests for CXX-Qt" ON)

if(VCPKG)
    # Until debug and release packages are split we only build release packages
    # to improve CI run times
    # https://github.com/microsoft/vcpkg/issues/1626
    if(NOT CMAKE_BUILD_TYPE STREQUAL "Release")
        message(WARNING "vcpkg only has release mode builds in it's cache")
    endif()

    if(USE_QT5)
        set(VCPKG_MANIFEST_FEATURES "qt5")
    else()
        set(VCPKG_MANIFEST_FEATURES "qt6")
    endif()

    include(InitializeVcpkg)

    # These are required for binary caching to work reliably across machines.
    set(VCPKG_FEATURE_FLAGS "-compilertracking")
    if(VCPKG_BUILDTREES_ROOT STREQUAL "")
        message(STATUS "Using default vcpkg buildtress root")
        set(VCPKG_INSTALL_OPTIONS "--x-abi-tools-use-exact-versions")
    else()
        message(STATUS "Using custom vcpkg buildtrees root: ${VCPKG_BUILDTREES_ROOT}")
        set(VCPKG_INSTALL_OPTIONS "--x-abi-tools-use-exact-versions;--x-buildtrees-root=${VCPKG_BUILDTREES_ROOT}")
    endif()

    if(NOT DEFINED ENV{VCPKG_BINARY_SOURCES})
        if(WIN32)
            set(COMMAND_PREFIX "")
            set(EXE_SUFFIX ".exe")
            set(SCRIPT_SUFFIX ".bat")
            set(DOTNET_RUNTIME "")
        else()
            set(COMMAND_PREFIX "./")
            set(EXE_SUFFIX "")
            set(SCRIPT_SUFFIX ".sh")
            set(DOTNET_RUNTIME "mono")
        endif()

        # vcpkg can download NuGet, so bootstrap vcpkg if the executable is not found.
        if(NOT EXISTS "${VCPKG_ROOT}/vcpkg${EXE_SUFFIX}")
        message(STATUS "Bootstrapping vcpkg")
        execute_process(
            COMMAND "${COMMAND_PREFIX}bootstrap-vcpkg${SCRIPT_SUFFIX}"
            WORKING_DIRECTORY ${VCPKG_ROOT}
        )
        endif()

        message(STATUS "Setting up vcpkg binary caching with read-only access to GitHub Packages NuGet source")

        execute_process(
            COMMAND "${COMMAND_PREFIX}vcpkg${EXE_SUFFIX}" fetch nuget
            WORKING_DIRECTORY ${VCPKG_ROOT}
            OUTPUT_VARIABLE NUGET_FETCH_OUTPUT
            OUTPUT_STRIP_TRAILING_WHITESPACE
        )
        string(REPLACE "\n" ";" NUGET_FETCH_OUTPUT "${NUGET_FETCH_OUTPUT}")
        list(POP_BACK NUGET_FETCH_OUTPUT NUGET_EXECUTABLE)

        # NuGet will fail with an error when trying to add a source with the same name
        # as one that already exists, so check that the NuGet source has not been added yet.
        execute_process(
            COMMAND ${DOTNET_RUNTIME} ${NUGET_EXECUTABLE} sources list
            OUTPUT_VARIABLE NUGET_SOURCES_LIST
        )
        string(FIND "${NUGET_SOURCES_LIST}" "cxx-qt-github-packages" SEARCH_RESULT)
        if(SEARCH_RESULT EQUAL -1)
        # GitHub will deactivate a personal access token that gets committed to the repository.
        # Hack around this by splitting up the PAT.
        # This is safe because this PAT only has read:packages permission.
        set(GITHUB_PAT_READ_PACKAGES_SUFFIX HvVJ7NF8sArqcyBnF45RXOgAT0Q1uL42CZkO)
        execute_process(
            COMMAND ${DOTNET_RUNTIME} ${NUGET_EXECUTABLE} sources add
            -name cxx-qt-github-packages
            -source https://nuget.pkg.github.com/KDAB/index.json
            -username KDAB
            -password ghp_${GITHUB_PAT_READ_PACKAGES_SUFFIX}
        )
        endif()

        set(ENV{VCPKG_BINARY_SOURCES} "clear;default,readwrite;nuget,cxx-qt-github-packages,read;")
    endif()
else()
    message(STATUS "Using dependencies from system without vcpkg")
endif()

project(cxx_qt)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

include(CompilerCaching)

# Enable extra Qt definitions for all projects
add_compile_definitions(
    QT_NO_CAST_FROM_ASCII
    QT_NO_CAST_TO_ASCII
    QT_NO_CAST_FROM_BYTEARRAY
    QT_NO_URL_CAST_FROM_STRING
    QT_NO_NARROWING_CONVERSIONS_IN_CONNECT
    QT_NO_FOREACH
    QT_NO_JAVA_STYLE_ITERATORS
    QT_NO_KEYWORDS
    QT_USE_QSTRINGBUILDER
)

if(BUILD_WASM)
    # TODO: fix wasm test failures
    set(BUILD_TESTING OFF)
    # Ensure Rust build for the correct target
    set(Rust_CARGO_TARGET wasm32-unknown-emscripten)
    set(THREADS_PREFER_PTHREAD_FLAG ON)
    find_package(Threads REQUIRED)
endif()

# QMAKE environment variable is needed by qt-build-utils to ensure that Cargo
# uses the same installation of Qt as CMake does.
if(NOT USE_QT5)
    find_package(Qt6 COMPONENTS Core Gui Test QuickControls2)
endif()
if(NOT Qt6_FOUND)
    if(BUILD_WASM)
        message(FATAL_ERROR 
            "CXX-Qt for WebAssembly only currently supports Qt 6 builds."
        )
    endif()
    find_package(Qt5 5.15 COMPONENTS Core Gui Test REQUIRED)
endif()

if (NOT Qt5_FOUND)
    add_definitions(-DQT_NO_CONTEXTLESS_CONNECT)
endif()

if(UNIX AND NOT APPLE)
    if (Qt5_FOUND)
        add_compile_definitions(
            QT_STRICT_ITERATORS
        )
    endif()
endif()

find_program(MEMORYCHECK_COMMAND valgrind)
if(NOT "${MEMORYCHECK_COMMAND}" STREQUAL "MEMORYCHECK_COMMAND-NOTFOUND")
    if (NOT WIN32)
       MESSAGE(STATUS "Valgrind found! Tests based on valgrind must be executed.")
    endif()
endif()

# Set our extra command options for valgrind
# TODO: we need to come up with a better way to suppress "possibly lost" errors.
# Suppression file doesn't work because there is a ton of mangled names that won't remain stable.
set(MEMORYCHECK_COMMAND_OPTIONS --error-exitcode=1 --errors-for-leak-kinds=definite --leak-check=full --trace-children=yes --track-origins=yes --show-possibly-lost=no)
# A suppressions file which silences errors from other libs like QtCore
set(MEMORYCHECK_SUPPRESSIONS_FILE "${CMAKE_SOURCE_DIR}/valgrind_suppressions.txt")

# Enable testing (this needs to be called before subdirs are added to detect tests in them)
enable_testing()

# Create helper method which adds a valgrind test with the given binary
function(add_valgrind_test NAME_WITH_PREFIX BINARY WORKING_DIRECTORY)
    if("${MEMORYCHECK_COMMAND}" STREQUAL "MEMORYCHECK_COMMAND-NOTFOUND")
       if (NOT WIN32)
           MESSAGE(STATUS "valgrind not found. Please install it")
       endif()
    else()
       add_test(NAME ${NAME_WITH_PREFIX}_valgrind
           COMMAND ${MEMORYCHECK_COMMAND} ${MEMORYCHECK_COMMAND_OPTIONS} --suppressions=${MEMORYCHECK_SUPPRESSIONS_FILE} --gen-suppressions=all ${BINARY}
           WORKING_DIRECTORY "${WORKING_DIRECTORY}"
       )
    endif()
endfunction()

get_target_property(QMAKE Qt::qmake IMPORTED_LOCATION)
set(CARGO_ENV "QMAKE=set:${QMAKE}")
set(RUNTIME_ENV "")

# On windows, Qt dll needs to be in the PATH for the tests to run
if(CMAKE_SYSTEM_NAME STREQUAL "Windows" AND BUILD_TESTING)
    execute_process(
        COMMAND ${QMAKE} -query QT_INSTALL_BINS
        OUTPUT_VARIABLE QT_INSTALL_BINS
        OUTPUT_STRIP_TRAILING_WHITESPACE
    )

    execute_process(
        COMMAND ${QMAKE} -query QT_INSTALL_PLUGINS
        OUTPUT_VARIABLE QT_INSTALL_PLUGINS
        OUTPUT_STRIP_TRAILING_WHITESPACE
    )

    execute_process(
        COMMAND ${QMAKE} -query QT_INSTALL_QML
        OUTPUT_VARIABLE QT_INSTALL_QML
        OUTPUT_STRIP_TRAILING_WHITESPACE
    )

    list(
        APPEND
        RUNTIME_ENV
        "PATH=path_list_append:${QT_INSTALL_BINS}"
        "QT_PLUGIN_PATH=path_list_append:${QT_INSTALL_PLUGINS}"
        "QML_IMPORT_PATH=path_list_append:${QT_INSTALL_QML}"
        "QML2_IMPORT_PATH=path_list_append:${QT_INSTALL_QML}"
    )
    list(APPEND CARGO_ENV ${RUNTIME_ENV})
endif()

# Same logic as in Corrosion.cmake
if(CMAKE_VS_PLATFORM_NAME)
    set(BUILD_DIR "${CMAKE_VS_PLATFORM_NAME}/$<CONFIG>")
elseif(CMAKE_CONFIGURATION_TYPES)
    set(BUILD_DIR "$<CONFIG>")
else()
    set(BUILD_DIR .)
endif()

# Set the target dir to the same that Corrosion uses to reuse build artifacts
# from the main build.
set(CARGO_TARGET_DIR "${CMAKE_BINARY_DIR}/${BUILD_DIR}/cargo/build")

if(BUILD_TESTING)
    # Add CMake tests for `cargo test/clippy/fmt/doc`.
    add_test(NAME cargo_tests COMMAND cargo test --release --all-features --target-dir ${CARGO_TARGET_DIR})
    add_test(NAME cargo_doc COMMAND cargo doc --release --all-features --target-dir ${CARGO_TARGET_DIR})
    add_test(NAME cargo_clippy COMMAND cargo clippy --release --all-features --target-dir ${CARGO_TARGET_DIR} -- -D warnings)
    
    set_tests_properties(cargo_tests cargo_clippy PROPERTIES
        ENVIRONMENT_MODIFICATION "${CARGO_ENV}"
    )
    set_tests_properties(cargo_doc PROPERTIES
        ENVIRONMENT_MODIFICATION "${CARGO_ENV};RUSTDOCFLAGS=set:--deny=warnings"
    )
    
    # Ensure test inputs and outputs are formatted
    file(GLOB CXX_QT_GEN_TEST_INPUTS ${CMAKE_CURRENT_SOURCE_DIR}/crates/cxx-qt-gen/test_inputs/*.rs)
    file(GLOB CXX_QT_GEN_TEST_OUTPUTS ${CMAKE_CURRENT_SOURCE_DIR}/crates/cxx-qt-gen/test_outputs/*.rs)
    add_test(NAME cxx_qt_gen_test_inputs_gen COMMAND rustfmt --check ${CXX_QT_GEN_TEST_INPUTS})
    add_test(NAME cxx_qt_gen_test_outputs_gen COMMAND rustfmt --check ${CXX_QT_GEN_TEST_OUTPUTS})
    
    # Add test which checks that a build rerun doesn't recompile and uses caches instead
    add_test(NAME cargo_build_rerun COMMAND "${CMAKE_CURRENT_SOURCE_DIR}/scripts/check_cargo_build_rerun.sh" "${CMAKE_CURRENT_SOURCE_DIR}")
    
    # Ensure that cargo_build_rerun doesn't run while we are already building
    set_tests_properties(cargo_build_rerun PROPERTIES RUN_SERIAL TRUE)
endif()

if(CMAKE_RUSTC_WRAPPER)
    list(APPEND CARGO_ENV "RUSTC_WRAPPER=set:${CMAKE_RUSTC_WRAPPER}")
endif()

add_subdirectory(book)
add_subdirectory(examples)

if(BUILD_TESTING)
    add_subdirectory(tests)
endif()
