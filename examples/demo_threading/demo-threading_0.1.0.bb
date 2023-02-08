# Auto-Generated by cargo-bitbake 0.3.16
#
inherit cargo

# If this is git based prefer versioned ones if they exist
# DEFAULT_PREFERENCE = "-1"

# how to get demo-threading could be as easy as but default to a git checkout:
# SRC_URI += "crate://crates.io/demo-threading/0.1.0"
SRC_URI += "gitsm://git@github.com/LeonMatthesKDAB/cxx-qt.git;protocol=ssh;nobranch=1;branch=yocto-kirkstone"
SRCREV = "c645cc12cb3187fc0848714426a12b47fe21eb99"
S = "${WORKDIR}/git"
CARGO_SRC_DIR = "examples/demo_threading"
PV:append = ".AUTOINC+c645cc12cb"

# please note if you have entries that do not begin with crate://
# you must change them to how that package can be fetched
SRC_URI += " \
    crate://crates.io/ansi_term/0.12.1 \
    crate://crates.io/async-channel/1.7.1 \
    crate://crates.io/async-executor/1.4.1 \
    crate://crates.io/async-global-executor/2.2.0 \
    crate://crates.io/async-io/1.8.0 \
    crate://crates.io/async-lock/2.5.0 \
    crate://crates.io/async-std/1.12.0 \
    crate://crates.io/async-task/4.3.0 \
    crate://crates.io/atomic-waker/1.0.0 \
    crate://crates.io/autocfg/1.1.0 \
    crate://crates.io/blocking/1.2.0 \
    crate://crates.io/bumpalo/3.11.0 \
    crate://crates.io/cache-padded/1.2.0 \
    crate://crates.io/cc/1.0.73 \
    crate://crates.io/cfg-if/1.0.0 \
    crate://crates.io/clang-format/0.1.2 \
    crate://crates.io/codespan-reporting/0.11.1 \
    crate://crates.io/concurrent-queue/1.2.4 \
    crate://crates.io/convert_case/0.6.0 \
    crate://crates.io/crossbeam-utils/0.8.11 \
    crate://crates.io/ctor/0.1.23 \
    crate://crates.io/cxx-build/1.0.86 \
    crate://crates.io/cxx-gen/0.7.86 \
    crate://crates.io/cxx/1.0.86 \
    crate://crates.io/cxxbridge-flags/1.0.86 \
    crate://crates.io/cxxbridge-macro/1.0.86 \
    crate://crates.io/diff/0.1.13 \
    crate://crates.io/either/1.8.0 \
    crate://crates.io/event-listener/2.5.3 \
    crate://crates.io/fastrand/1.8.0 \
    crate://crates.io/futures-channel/0.3.23 \
    crate://crates.io/futures-core/0.3.23 \
    crate://crates.io/futures-executor/0.3.23 \
    crate://crates.io/futures-io/0.3.23 \
    crate://crates.io/futures-lite/1.12.0 \
    crate://crates.io/futures-macro/0.3.23 \
    crate://crates.io/futures-sink/0.3.23 \
    crate://crates.io/futures-task/0.3.23 \
    crate://crates.io/futures-timer/3.0.2 \
    crate://crates.io/futures-util/0.3.23 \
    crate://crates.io/futures/0.3.23 \
    crate://crates.io/getrandom/0.2.7 \
    crate://crates.io/gloo-timers/0.2.4 \
    crate://crates.io/hermit-abi/0.1.19 \
    crate://crates.io/indoc/1.0.7 \
    crate://crates.io/instant/0.1.12 \
    crate://crates.io/itertools/0.10.3 \
    crate://crates.io/itoa/1.0.3 \
    crate://crates.io/jobserver/0.1.24 \
    crate://crates.io/js-sys/0.3.59 \
    crate://crates.io/kv-log-macro/1.0.7 \
    crate://crates.io/libc/0.2.132 \
    crate://crates.io/link-cplusplus/1.0.7 \
    crate://crates.io/log/0.4.17 \
    crate://crates.io/memchr/2.5.0 \
    crate://crates.io/minimal-lexical/0.2.1 \
    crate://crates.io/nom/7.1.1 \
    crate://crates.io/num_cpus/1.13.1 \
    crate://crates.io/once_cell/1.13.1 \
    crate://crates.io/output_vt100/0.1.3 \
    crate://crates.io/parking/2.0.0 \
    crate://crates.io/pin-project-lite/0.2.9 \
    crate://crates.io/pin-utils/0.1.0 \
    crate://crates.io/polling/2.3.0 \
    crate://crates.io/pretty_assertions/1.2.1 \
    crate://crates.io/proc-macro2/1.0.43 \
    crate://crates.io/quote/1.0.21 \
    crate://crates.io/ryu/1.0.11 \
    crate://crates.io/scratch/1.0.2 \
    crate://crates.io/serde/1.0.144 \
    crate://crates.io/serde_derive/1.0.144 \
    crate://crates.io/serde_json/1.0.85 \
    crate://crates.io/slab/0.4.7 \
    crate://crates.io/socket2/0.4.4 \
    crate://crates.io/syn/1.0.99 \
    crate://crates.io/termcolor/1.1.3 \
    crate://crates.io/thiserror-impl/1.0.32 \
    crate://crates.io/thiserror/1.0.32 \
    crate://crates.io/unicode-ident/1.0.3 \
    crate://crates.io/unicode-segmentation/1.10.0 \
    crate://crates.io/unicode-width/0.1.9 \
    crate://crates.io/uuid/1.2.2 \
    crate://crates.io/value-bag/1.0.0-alpha.9 \
    crate://crates.io/version_check/0.9.4 \
    crate://crates.io/versions/4.1.0 \
    crate://crates.io/waker-fn/1.1.0 \
    crate://crates.io/wasi/0.11.0+wasi-snapshot-preview1 \
    crate://crates.io/wasm-bindgen-backend/0.2.82 \
    crate://crates.io/wasm-bindgen-futures/0.4.32 \
    crate://crates.io/wasm-bindgen-macro-support/0.2.82 \
    crate://crates.io/wasm-bindgen-macro/0.2.82 \
    crate://crates.io/wasm-bindgen-shared/0.2.82 \
    crate://crates.io/wasm-bindgen/0.2.82 \
    crate://crates.io/web-sys/0.3.59 \
    crate://crates.io/wepoll-ffi/0.1.2 \
    crate://crates.io/winapi-i686-pc-windows-gnu/0.4.0 \
    crate://crates.io/winapi-util/0.1.5 \
    crate://crates.io/winapi-x86_64-pc-windows-gnu/0.4.0 \
    crate://crates.io/winapi/0.3.9 \
"



# FIXME: update generateme with the real MD5 of the license file
LIC_FILES_CHKSUM = " \
    file://LICENSES;md5=generateme \
    file://MIT.txt;md5=generateme \
"

SUMMARY = "demo-threading"
HOMEPAGE = "https://github.com/LeonMatthesKDAB/cxx-qt.git"
LICENSE = "LICENSES | MIT.txt"

# includes this file if it exists but does not fail
# this is useful for anything you may want to override from
# what cargo-bitbake generates.
include demo-threading-${PV}.inc
include demo-threading.inc
