// Taken directly from https://github.com/domeengine/dome/blob/ffc47ca273430c2da0b0479ab12f959e57d12ba9/examples/plugin/main.wren

// Run using:
// ```
// cargo build --example first
// cp ../../target/debug/examples/first.ext first.ext
// dome
// ```
// Where `.ext` is the extension of dynamic library on your platform:
// `.dll` on Windows, `.so` on Linux, and `.dylib` on MacOS.

import "plugin" for Plugin

Plugin.load("first")
// The plugin will be initialised now

// Plugins can register their own modules
import "external" for ExternalClass


class Game {
    static init() {
      // and allocators for foreign classes
      var obj = ExternalClass.init()

      // and finally, they can register foreign methods implemented
      // in the plugin native language.
      obj.alert("Some words")
    }
    static update() {}
    static draw(dt) {}
}
