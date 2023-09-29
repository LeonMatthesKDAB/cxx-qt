<!--
SPDX-FileCopyrightText: 2021 Klarälvdalens Datakonsult AB, a KDAB Group company <info@kdab.com>
SPDX-FileContributor: Andrew Hayzen <andrew.hayzen@kdab.com>

SPDX-License-Identifier: MIT OR Apache-2.0
-->

# CXX-Qt - Safe interop between Rust and Qt

<p align=center><a href="./getting-started/index.md">TLDR: Click here for "Getting Started" guide</a></p>

This library provides a safe mechanism for bridging between Qt code and Rust code in a different way to typical Rust Qt bindings.

We acknowledge that Qt code and Rust code have different idioms so cannot be directly wrapped from one to another.
Qt extends C++ object-based system with their own meta-object system, which is a reflection layer on top of normal C++ constructs that allows for runtime reflection.

Instead of one-to-one bindings we extend [CXX](https://cxx.rs/) to [bridge](./bridge/index.md) between Qt and Rust code, this allows for normal Qt code and normal Rust code.
The CXX-Qt bridge allows defining Qt meta-objects (think QObjects with properties, etc.) that are backed by Rust code.
Additionally, existing meta-objects can be exposed to Rust as if they were normal Rust structs, thereby allowing two-way communication between Rust and Qt code.

In comparison to typical bindings through a C FFI, this empowers Rust users by providing a way to easily define Qt constructs in Rust and to reference existing Qt constructs from Rust.

As seen in the figure below, the developer describes Qt meta-objects using CXX-Qt macro annotations.
CXX-Qt then generates the necessary code to interact with the described meta-objects from both C++ and Rust.
Internally, CXX-Qt uses CXX for the heavy lifting and allows using CXX concepts directly from within a CXX-Qt bridge.

<div style="background-color: white; padding: 1rem; text-align: center;">

![Overview of CXX-Qt concept](./images/overview_abstract.svg)

</div>

CXX-Qt also comes with a library of pre-existing bindings for fundamental Qt types.
This library is described in the [types page](./concepts/types.md) and distributed through the [cxx-qt-lib crate](https://docs.rs/cxx-qt-lib/).

If you are new to CXX-Qt, we recommend you visit our [Getting Started Guide](./getting-started/index.md).

To get detailed information on which features are available in CXX-Qt, see the [bridge chapter](./bridge/index.md).
Should you be interested in a deeper dive into the concepts of CXX-Qt, take a look at the [concepts chapter](./concepts/index.md), which explains the concepts CXX-Qt introduces in detail.

**Note:** CXX-Qt is tested on CI on Linux, Windows, and macOS (all on x86_64). It should work on other platforms that Qt and Rust both support, however, these are not tested regularly.
