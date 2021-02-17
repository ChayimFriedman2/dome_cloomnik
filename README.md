# [DOME Cloomnik](https://docs.rs/dome_cloomnik)

A Rust framework for building DOME plugins.

The basic structure of every plugin using this framework is:

Cargo.toml:

```toml
[package]
name = "my_awesome_dome_plugin"
description = "Really, really awesome DOME plugin written in Rust!"
version = "0.1.0"
authors = ["Me <me@gmail.com>"]
edition = "2018"

[dependencies]
libc = "0.2"
dome_cloomnik = "0.1"

[lib]
crate-type = ["cdylib"]
```

lib.rs:

```rust
use dome_cloomnik::{Context, WrenVM, register_modules, HookResult};

#[no_mangle]
#[allow(non_snake_case)]
extern "C" fn PLUGIN_onInit(get_api: *mut libc::c_void, ctx: *mut libc::c_void) -> libc::c_int {
    unsafe {
        dome_cloomnik::init_plugin(
            get_api,
            ctx,
            dome_cloomnik::Hooks {
                on_init: Some(on_init),
                pre_update: Some(pre_update),
                post_update: Some(post_update),
                pre_draw: Some(pre_draw),
                post_draw: Some(post_draw),
                on_shutdown: Some(on_shutdown),
            }
        )
    }
}

fn on_init(mut ctx: Context) -> HookResult {
    (register_modules! {
        ctx,
        ...
    })?;

    // ...
}

fn pre_update(mut ctx: Context) -> HookResult {
    // ...
}

fn post_update(mut ctx: Context) -> HookResult {
    // ...
}

fn pre_draw(mut ctx: Context) -> HookResult {
    // ...
}

fn post_draw(mut ctx: Context) -> HookResult {
    // ...
}

fn on_shutdown(mut ctx: Context) -> HookResult {
    // ...
}
```

Go ahead, and start with [learning DOME plugins from the docs](https://domeengine.com/plugins/).
Don't worry, much of the things there will apply to doom_cloomnik too!
