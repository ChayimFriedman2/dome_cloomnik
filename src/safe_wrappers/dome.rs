use std::ffi::CString;
use std::marker::PhantomData;
use std::mem;

use super::audio;
use super::wren;
use crate::unsafe_wrappers::dome as unsafe_dome;
use crate::Api;

type Result = std::result::Result<(), ()>;

pub(crate) type ForeignFn = wren::ForeignMethodFn;
pub(crate) type FinalizerFn = wren::FinalizerFn;

/// A context is the gate to all of DOME's functionalities for plugins.
///
/// You get a context for each callback, and you can retrieve it from
/// [`WrenVM`][crate::WrenVM] via [`get_context()`][crate::WrenVM::get_context()], for use
/// in foreign methods.
///
/// Note that you should not keep a context while giving the control back to DOME,
/// and this is enforced by Rust's ownership system.
#[derive(Debug)]
#[repr(transparent)]
pub struct Context<'a>(
    pub(crate) unsafe_dome::Context,
    pub(crate) PhantomData<&'a ()>,
);

impl Context<'_> {
    /// Register a Wren module that Wren code can import and use the functionalities
    /// it provides.
    ///
    /// This functions fails if there is already a module with the same name.
    ///
    /// It is recommended to use the [`register_modules!`] macro instead.
    ///
    /// # Example
    ///
    /// ```
    /// # let ctx: Context;
    /// ctx.register_module("my-module", r#"
    ///     class MyClass {}
    /// "#)
    /// .unwrap();
    /// ```
    #[inline]
    pub fn register_module(&self, name: &str, source: &str) -> Result {
        let name = CString::new(name).expect("Module name contains null byte(s).");
        let source = CString::new(source).expect("Source contains null byte(s).");
        (Api::dome().register_module)(self.0, name.as_ptr(), source.as_ptr()).into()
    }

    /// Register a foreign method in `module` with `signature` of the following form:
    /// ```wren
    /// [static ]ClassName.wrenSignature
    /// ```
    /// For more information about DOME signatures, see [DOME docs](https://domeengine.com/plugins/#method-registerfn).
    ///
    /// For more information about Wren signatures, see [Wren docs](https://wren.io/method-calls.html#signature).
    ///
    /// Fails if the same method is already registered.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it is forbidden to register a function that may panic,
    /// and/or store the VM instance it gets for later use.
    ///
    /// Doing so may lead to Undefined Behavior.
    ///
    /// It is recommended to use the [`register_modules!`] macro instead, which also solves
    /// the above concerns.
    ///
    /// # Example
    ///
    /// ```
    /// # let ctx: Context;
    /// ctx.register_module("my-module", r#"
    ///     class MyClass {
    ///         foreign myGetter
    ///     }
    /// "#)
    /// .unwrap();
    /// extern "C" fn my_fn(vm: WrenVM) {}
    /// # unsafe {
    /// ctx.register_fn("my-module", "MyClass.myGetter", my_fn).unwrap();
    /// # }
    /// ```
    #[inline]
    pub unsafe fn register_fn(&self, module: &str, signature: &str, method: ForeignFn) -> Result {
        let module = CString::new(module).expect("Method name contains null byte(s).");
        let signature = CString::new(signature).expect("Method signature contains null byte(s).");
        (Api::dome().register_fn)(
            self.0,
            module.as_ptr(),
            signature.as_ptr(),
            mem::transmute(method),
        )
        .into()
    }

    /// Register a foreign class in `module` with `allocate` and possibly `finalizer`:
    ///
    /// Fails if the same class is already registered.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it is forbidden to register a function that may panic,
    /// and/or store the VM instance it gets for later use, and/or an allocator that does not
    /// call [`WrenVM::set_slot_new_raw_foreign_unchecked()`] or [`WrenVM::set_slot_new_foreign_unchecked()`].
    ///
    /// Doing so may lead to Undefined Behavior.
    ///
    /// It is recommended to use the [`register_modules!`] macro instead, which also solves
    /// the above concerns.
    ///
    /// # Example
    ///
    /// ```
    /// # let ctx: Context;
    /// ctx.register_module("my-module", r#"
    ///     foreign class MyClass {
    ///         construct new() {}
    ///     }
    /// "#)
    /// .unwrap();
    /// extern "C" fn allocate(vm: WrenVM) {}
    /// extern "C" fn finalize(data: *mut libc::c_void) {}
    /// # unsafe {
    /// ctx.register_class("my-module", "MyClass", allocate, Some(finalize)).unwrap();
    /// # }
    /// ```
    #[inline]
    pub unsafe fn register_class(
        &self,
        module_name: &str,
        class_name: &str,
        allocate: ForeignFn,
        finalize: Option<FinalizerFn>,
    ) -> Result {
        let module_name = CString::new(module_name).expect("Module name contains null byte(s).");
        let class_name = CString::new(class_name).expect("Class name contains null byte(s).");
        (Api::dome().register_class)(
            self.0,
            module_name.as_ptr(),
            class_name.as_ptr(),
            mem::transmute(allocate),
            finalize,
        )
        .into()
    }

    /// Locks a module, preventing extending it later.
    ///
    /// It is recommended to lock all modules after you finished to register all
    /// of their members.
    ///
    /// It is even more recommended to use [`register_modules!`], which does so automatically.
    #[inline]
    pub fn lock_module(&self, name: &str) {
        let name = CString::new(name).expect("Module name contains null byte(s).");
        (Api::dome().lock_module)(self.0, name.as_ptr())
    }

    /// Logs text to the DOME-out.log file and possibly to the console.
    #[inline]
    pub fn log(&self, text: &str) {
        let fmt = CString::new("%s").unwrap();
        let text = CString::new(text).expect("Text contains null byte(s).");
        unsafe { (Api::dome().log)(self.0, fmt.as_ptr(), text.as_ptr()) }
    }

    /// Creates a new audio channel, with user data of type `T`.
    ///
    /// `mix`: A callback that is responsible to generate the next frame.
    ///
    /// `update`: A callback to be called in the free time.
    ///
    /// `user_data`: The user data. Can be retrieved later using [`Channel::data()`][crate::Channel]
    /// and [`Channel::data_mut()`] (and counterparts in [`CallbackChannel`][crate::CallbackChannel]).
    ///
    /// The user data must be safe to transfer and share across threads, because `mix`
    /// is executed on another thread. If you don't need data, you can just use [the unit type](https://doc.rust-lang.org/std/primitive.unit.html).
    ///
    /// The returned channel is automatically stopped on drop. Use [`mem::forget()`] if that
    /// isn't the intention.
    #[inline]
    pub fn create_channel<T: Send + Sync>(
        &self,
        mix: audio::ChannelMix<T>,
        update: audio::ChannelUpdate<T>,
        user_data: T,
    ) -> audio::Channel<T> {
        let data = Box::into_raw(Box::new(audio::ChannelData::new(mix, update, user_data)));
        audio::Channel(
            (Api::audio().channel_create)(
                self.0,
                audio::mix,
                audio::update,
                if mem::needs_drop::<T>() {
                    audio::finish
                } else {
                    audio::finish_no_drop
                },
                data as *mut _,
            ),
            PhantomData,
        )
    }
}

/// Helper macro to register modules in Wren.
///
/// **NOTE**: Do NOT depend on destructors! They run nondeterministically
/// when the GC thinks it's appropriate. Provide your users a `close()`
/// method or similar mechanism to free resources! Destructors of foreign
/// objects should be used solely for:
///
///   1. Freeing _memory_ owned by this type.
///   2. Cleaning resources that has not been cleaned, probably forgotten
///      by the Wren programmer.
///
/// Note that Rust's ownership system means that you may hold an object
/// that its destructor will run even when it is already closed, and can
/// even be deeply nested. To solve that, either:
///
///  - Ensure that your objects won't get bad state if dropped twice.
///    This usually mean some sort of guard in their destructor.
///  - (Especially good if you do not have control over the element,
///    or even it does not have a `close()` method at all, for example
///    objects from the standard library:) Hold an `Option<Object>`
///    instead of plain `Object` and set it to `None` in `close()`.
///    This way, you run the destructor automatically, and the object
///    won't be closed again.
///
/// # Example
/// ```rust
/// struct MyType;
/// impl MyType {
///     fn new(vm: &wren::VM) -> Self {
///         MyType
///     }
///     fn foreign_subscript_setter(vm: &wren::VM) {}
/// }
/// #[derive(Debug)]
/// struct MyOtherType(f64);
/// impl MyOtherType {
///     fn construct(vm: &wren::VM) -> Self {
///         MyOtherType(vm.get_slot_double(1))
///     }
///     fn foreign_method(&mut self, vm: &wren::VM) {}
/// }
/// impl Drop for MyOtherType {
///     fn drop(&mut self) {
///         println!("MyOtherType's destructor: {:?}", self);
///     }
/// }
/// mod non_foreign {
///     pub(super) struct SomeOtherClass;
///     impl SomeOtherClass {
///         pub(super) fn foreign_getter(vm: &crate::WrenVM) {}
///     }
/// }
/// register_modules! {
///     ctx,
///     module "my-module" {
///         foreign class MyClass = new of MyType {
///             "construct new() { }"
///             foreign static [a, b]=(value) = foreign_subscript_setter
///         }
///         foreign class MyOtherClass is "MyClass" = construct of MyOtherType {
///             "construct new(shouldBeDouble) { }"
///             foreign method() = foreign_method
///         }
///         class SomeOtherClass = non_foreign::SomeOtherClass {
///             r#"
///             construct myConstructor(p1, p2) {
///                 _f1 = p1
///                 _f2 = p2
///             }
///             getter { "Wren code here" }
///             "#
///             foreign foreignGetter = foreign_getter
///             "method() {
///                 System.print([1, 2, 3])
///             }"
///         }
///     }
///     module "my-second-module" {}
/// }
/// ```
#[macro_export]
macro_rules! register_modules {
    { $ctx:expr, $($modules:tt)+ } => {{
        $crate::register_modules! { @process_modules
            ctx = [{ $ctx }]
            modules = [{ $($modules)+ }]
        }
    }};

    { @process_modules
        ctx = [{ $ctx:expr }]
        modules = [{ $(
            module $module_name:literal { $($module_contents:tt)* }
        )+ }]
    } => {
        $(
            $ctx.register_module(
                $module_name,
                $crate::register_modules! { @get_module_source
                    items = [{ $($module_contents)* }]
                }
            ).unwrap();

            $crate::register_modules! { @register_module_members
                ctx = [{ $ctx }]
                module = [{ $module_name }]
                items = [{ $($module_contents)* }]
            }

            $ctx.lock_module($module_name);
        )+
    };

    { @get_module_source
        items = [{ }]
    } => { "" };
    { @get_module_source
        items = [{
            foreign class $name:ident $(is $superclass:literal)? = $constructor:ident of $foreign_type:ty {
                $($class_contents:tt)*
            }
            $($rest:tt)*
        }]
    } => {
        concat!(
            "foreign class ", stringify!($name), $(" is (", $superclass, ")",)? " {\n",
            $crate::register_modules! { @get_class_source
                items = [{ $($class_contents)* }]
            },
            "}\n",
            $crate::register_modules! { @get_module_source
                items = [{ $($rest)* }]
            },
        )
    };
    { @get_module_source
        items = [{
            class $name:ident $(is $superclass:literal)? = $type:ty { $($class_contents:tt)* }
            $($rest:tt)*
        }]
    } => {
        concat!(
            "class ", stringify!($name), $(" is (", $superclass, ")",)? " {\n",
            $crate::register_modules! { @get_class_source
                items = [{ $($class_contents)* }]
            },
            "}\n",
            $crate::register_modules! { @get_module_source
                items = [{ $($rest)* }]
            },
        )
    };

    { @get_class_source
        items = [{ }]
    } => { "" };
    // Static getter
    { @get_class_source
        items = [{
            foreign static $name:ident = $method:ident
            $($rest:tt)*
        }]
    } => {
        concat!(
            "foreign static ", stringify!($name), "\n",
            $crate::register_modules! { @get_class_source
                items = [{ $($rest)* }]
            },
        )
    };
    // Instance getter
    { @get_class_source
        items = [{
            foreign $name:ident = $method:ident
            $($rest:tt)*
        }]
    } => {
        concat!(
            "foreign ", stringify!($name), "\n",
            $crate::register_modules! { @get_class_source
                items = [{ $($rest)* }]
            },
        )
    };
    // Static setter
    { @get_class_source
        items = [{
            foreign static $name:ident=($value:ident) = $method:ident
            $($rest:tt)*
        }]
    } => {
        concat!(
            "foreign static ", stringify!($name), "=(", stringify!($value), ")\n",
            $crate::register_modules! { @get_class_source
                items = [{ $($rest)* }]
            },
        )
    };
    // Instance setter
    { @get_class_source
        items = [{
            foreign $name:ident=($value:ident) = $method:ident
            $($rest:tt)*
        }]
    } => {
        concat!(
            "foreign ", stringify!($name), "=(", stringify!($value), ")\n",
            $crate::register_modules! { @get_class_source
                items = [{ $($rest)* }]
            },
        )
    };
    // Static method
    { @get_class_source
        items = [{
            foreign static $name:ident($($param0:ident $(, $params:ident)*)?) = $method:ident
            $($rest:tt)*
        }]
    } => {
        concat!(
            "foreign static ", stringify!($name), "(",
                $(stringify!($param0), $(",", stringify!($params),)*)?
            ")\n",
            $crate::register_modules! { @get_class_source
                items = [{ $($rest)* }]
            },
        )
    };
    // Instance method
    { @get_class_source
        items = [{
            foreign $name:ident($($param0:ident $(, $params:ident)*)?) = $method:ident
            $($rest:tt)*
        }]
    } => {
        concat!(
            "foreign ", stringify!($name), "(",
                $(stringify!($param0), $(",", stringify!($params),)*)?
            ")\n",
            $crate::register_modules! { @get_class_source
                items = [{ $($rest)* }]
            },
        )
    };
    // Static subscript getter
    { @get_class_source
        items = [{
            foreign static [$param0:ident $(, $params:ident)*] = $method:ident
            $($rest:tt)*
        }]
    } => {
        concat!(
            "foreign static [", stringify!($param0), $(",", stringify!($params),)* "]\n",
            $crate::register_modules! { @get_class_source
                items = [{ $($rest)* }]
            },
        )
    };
    // Instance subscript getter
    { @get_class_source
        items = [{
            foreign [$param0:ident $(, $params:ident)*] = $method:ident
            $($rest:tt)*
        }]
    } => {
        concat!(
            "foreign [", stringify!($param0), $(",", stringify!($params),)* "]\n",
            $crate::register_modules! { @get_class_source
                items = [{ $($rest)* }]
            },
        )
    };
    // Static subscript setter
    { @get_class_source
        items = [{
            foreign static [$param0:ident $(, $params:ident)*]=($value:ident) = $method:ident
            $($rest:tt)*
        }]
    } => {
        concat!(
            "foreign static [",
                stringify!($param0), $(",", stringify!($params),)*
            "]=(", stringify!($value), ")\n",
            $crate::register_modules! { @get_class_source
                items = [{ $($rest)* }]
            },
        )
    };
    // Instance subscript setter
    { @get_class_source
        items = [{
            foreign [$param0:ident $(, $params:ident)*]=($value:ident) = $method:ident
            $($rest:tt)*
        }]
    } => {
        concat!(
            "foreign [",
                stringify!($param0), $(",", stringify!($params),)*
            "]=(", stringify!($value), ")\n",
            $crate::register_modules! { @get_class_source
                items = [{ $($rest)* }]
            },
        )
    };
    // Non-foreign method (of any kind)
    { @get_class_source
        items = [{
            $method:literal
            $($rest:tt)*
        }]
    } => {
        concat!(
            $method, "\n",
            $crate::register_modules! { @get_class_source
                items = [{ $($rest)* }]
            },
        )
    };

    { @register_module_members
        ctx = [{ $ctx:expr }]
        module = [{ $module:literal }]
        items = [{ }]
    } => { };
    { @register_module_members
        ctx = [{ $ctx:expr }]
        module = [{ $module:literal }]
        items = [{
            foreign class $name:ident $(is $superclass:literal)? = $constructor:ident of $foreign_type:ty {
                $($class_contents:tt)*
            }
            $($rest:tt)*
        }]
    } => {{
        extern "C" fn __dome_cloomnik_class_allocate(vm: $crate::WrenVM) {
            if let Some(instance) = $crate::__catch_panic_from_foreign(&vm, || {
                <$foreign_type>::$constructor(&vm)
            }) {
                unsafe {
                    vm.set_slot_new_foreign_unchecked(0, 0, instance);
                }
            }
        }
        #[allow(unused)]
        extern "C" fn __dome_cloomnik_class_finalize(data: *mut $crate::__c_void) {
            let data = data as *mut $crate::__ForeignWrapper<$foreign_type>;
            // We cannot report the failure, but we still have to not panic
            ::std::panic::catch_unwind(|| unsafe { ::std::ptr::drop_in_place(data) });
        }
        unsafe {
            $ctx.register_class(
                $module,
                stringify!($name),
                __dome_cloomnik_class_allocate,
                if ::std::mem::needs_drop::<$crate::__ForeignWrapper<$foreign_type>>() {
                    Some(__dome_cloomnik_class_finalize)
                } else {
                    None
                },
            )
        }
        .unwrap();
        $crate::register_modules! { @register_class_members
            ctx = [{ $ctx }]
            module = [{ $module }]
            class = [{ $name }]
            items = [{ $($class_contents)* }]
            type = [{ $foreign_type }]
            foreign_type = [{ $foreign_type }]
        }
        $crate::register_modules! { @register_module_members
            ctx = [{ $ctx }]
            module = [{ $module }]
            items = [{ $($rest)* }]
        }
    }};
    { @register_module_members
        ctx = [{ $ctx:expr }]
        module = [{ $module:literal }]
        items = [{
            class $name:ident $(is $superclass:literal)? = $type:ty { $($class_contents:tt)* }
            $($rest:tt)*
        }]
    } => {
        $crate::register_modules! { @register_class_members
            ctx = [{ $ctx }]
            module = [{ $module }]
            class = [{ $name }]
            items = [{ $($class_contents)* }]
            type = [{ $type }]
        }
        $crate::register_modules! { @register_module_members
            ctx = [{ $ctx }]
            module = [{ $module }]
            items = [{ $($rest)* }]
        }
    };

    // A little utility macro that allows us to replace parameter names by underscores
    // while still associating them to the repetition, so that `macro_rules!` won't complain
    { @underscore $($t:tt)* } => { "_" };

    { @register_class_members
        ctx = [{ $ctx:expr }]
        module = [{ $module:literal }]
        class = [{ $class:ident }]
        items = [{ }]
        type = [{ $($type:tt)+ }]
        $(foreign_type = [{ $($foreign_type:tt)+ }])?
    } => { };
    // Static getter
    { @register_class_members
        ctx = [{ $ctx:expr }]
        module = [{ $module:literal }]
        class = [{ $class:ident }]
        items = [{
            foreign static $name:ident = $method:ident
            $($rest:tt)*
        }]
        type = [{ $($type:tt)+ }]
        $(foreign_type = [{ $($foreign_type:tt)+ }])?
    } => {{
        extern "C" fn __dome_cloomnik_method(vm: $crate::WrenVM) {
            $crate::__catch_panic_from_foreign(&vm, || <$($type)+>::$method(&vm));
        }
        unsafe {
            $ctx.register_fn(
                $module,
                concat!("static ", stringify!($class), ".", stringify!($name)),
                __dome_cloomnik_method,
            )
        }
        .unwrap();
        $crate::register_modules! { @register_class_members
            ctx = [{ $ctx }]
            module = [{ $module }]
            class = [{ $class }]
            items = [{ $($rest)* }]
            type = [{ $($type)+ }]
            $(foreign_type = [{ $($foreign_type)+ }])?
        }
    }};
    // Instance getter
    { @register_class_members
        ctx = [{ $ctx:expr }]
        module = [{ $module:literal }]
        class = [{ $class:ident }]
        items = [{
            foreign $name:ident = $method:ident
            $($rest:tt)*
        }]
        type = [{ $($type:tt)+ }]
        $(foreign_type = [{ $($foreign_type:tt)+ }])?
    } => {{
        extern "C" fn __dome_cloomnik_method(vm: $crate::WrenVM) {
            $crate::__catch_panic_from_foreign(&vm, || {
                <$($type)+>::$method(
                    $(unsafe { vm.get_slot_foreign_unchecked::<$($foreign_type)+>(0) },)?
                    &vm,
                )
            });
        }
        unsafe {
            $ctx.register_fn(
                $module,
                concat!(stringify!($class), ".", stringify!($name)),
                __dome_cloomnik_method,
            )
        }
        .unwrap();
        $crate::register_modules! { @register_class_members
            ctx = [{ $ctx }]
            module = [{ $module }]
            class = [{ $class }]
            items = [{ $($rest)* }]
            type = [{ $($type)+ }]
            $(foreign_type = [{ $($foreign_type)+ }])?
        }
    }};
    // Static setter
    { @register_class_members
        ctx = [{ $ctx:expr }]
        module = [{ $module:literal }]
        class = [{ $class:ident }]
        items = [{
            foreign static $name:ident=($value:ident) = $method:ident
            $($rest:tt)*
        }]
        type = [{ $($type:tt)+ }]
        $(foreign_type = [{ $($foreign_type:tt)+ }])?
    } => {{
        extern "C" fn __dome_cloomnik_method(vm: $crate::WrenVM) {
            $crate::__catch_panic_from_foreign(&vm, || <$($type)+>::$method(&vm));
        }
        unsafe {
            $ctx.register_fn(
                $module,
                concat!("static ", stringify!($class), ".", stringify!($name), "=(_)"),
                __dome_cloomnik_method,
            )
        }
        .unwrap();
        $crate::register_modules! { @register_class_members
            ctx = [{ $ctx }]
            module = [{ $module }]
            class = [{ $class }]
            items = [{ $($rest)* }]
            type = [{ $($type)+ }]
            $(foreign_type = [{ $($foreign_type)+ }])?
        }
    }};
    // Instance setter
    { @register_class_members
        ctx = [{ $ctx:expr }]
        module = [{ $module:literal }]
        class = [{ $class:ident }]
        items = [{
            foreign $name:ident=($value:ident) = $method:ident
            $($rest:tt)*
        }]
        type = [{ $($type:tt)+ }]
        $(foreign_type = [{ $($foreign_type:tt)+ }])?
    } => {{
        extern "C" fn __dome_cloomnik_method(vm: $crate::WrenVM) {
            $crate::__catch_panic_from_foreign(&vm, || {
                <$($type)+>::$method(
                    $(unsafe { vm.get_slot_foreign_unchecked::<$($foreign_type)+>(0) },)?
                    &vm,
                )
            });
        }
        unsafe {
            $ctx.register_fn(
                $module,
                concat!(stringify!($class), ".", stringify!($name), "=(_)"),
                __dome_cloomnik_method,
            )
        }
        .unwrap();
        $crate::register_modules! { @register_class_members
            ctx = [{ $ctx }]
            module = [{ $module }]
            class = [{ $class }]
            items = [{ $($rest)* }]
            type = [{ $($type)+ }]
            $(foreign_type = [{ $($foreign_type)+ }])?
        }
    }};
    // Static method
    { @register_class_members
        ctx = [{ $ctx:expr }]
        module = [{ $module:literal }]
        class = [{ $class:ident }]
        items = [{
            foreign static $name:ident($($param0:ident $(, $params:ident)*)?) = $method:ident
            $($rest:tt)*
        }]
        type = [{ $($type:tt)+ }]
        $(foreign_type = [{ $($foreign_type:tt)+ }])?
    } => {{
        extern "C" fn __dome_cloomnik_method(vm: $crate::WrenVM) {
            $crate::__catch_panic_from_foreign(&vm, || <$($type)+>::$method(&vm));
        }
        unsafe {
            $ctx.register_fn(
                $module,
                concat!("static ", stringify!($class), ".", stringify!($name), "(",
                    $(
                        $crate::register_modules! { @underscore $param0 },
                        $(",", $crate::register_modules! { @underscore $params },)*
                    )?
                ")"),
                __dome_cloomnik_method,
            )
        }
        .unwrap();
        $crate::register_modules! { @register_class_members
            ctx = [{ $ctx }]
            module = [{ $module }]
            class = [{ $class }]
            items = [{ $($rest)* }]
            type = [{ $($type)+ }]
            $(foreign_type = [{ $($foreign_type)+ }])?
        }
    }};
    // Instance method
    { @register_class_members
        ctx = [{ $ctx:expr }]
        module = [{ $module:literal }]
        class = [{ $class:ident }]
        items = [{
            foreign $name:ident($($param0:ident $(, $params:ident)*)?) = $method:ident
            $($rest:tt)*
        }]
        type = [{ $($type:tt)+ }]
        $(foreign_type = [{ $($foreign_type:tt)+ }])?
    } => {{
        extern "C" fn __dome_cloomnik_method(vm: $crate::WrenVM) {
            $crate::__catch_panic_from_foreign(&vm, || {
                <$($type)+>::$method(
                    $(unsafe { vm.get_slot_foreign_unchecked::<$($foreign_type)+>(0) },)?
                    &vm,
                )
            });
        }
        unsafe {
            $ctx.register_fn(
                $module,
                concat!(stringify!($class), ".", stringify!($name), "(",
                    $(
                        $crate::register_modules! { @underscore $param0 },
                        $(",", $crate::register_modules! { @underscore $params },)*
                    )?
                ")"),
                __dome_cloomnik_method,
            )
        }
        .unwrap();
        $crate::register_modules! { @register_class_members
            ctx = [{ $ctx }]
            module = [{ $module }]
            class = [{ $class }]
            items = [{ $($rest)* }]
            type = [{ $($type)+ }]
            $(foreign_type = [{ $($foreign_type)+ }])?
        }
    }};
    // Static subscript getter
    { @register_class_members
        ctx = [{ $ctx:expr }]
        module = [{ $module:literal }]
        class = [{ $class:ident }]
        items = [{
            foreign static [$param0:ident $(, $params:ident)*] = $method:ident
            $($rest:tt)*
        }]
        type = [{ $($type:tt)+ }]
        $(foreign_type = [{ $($foreign_type:tt)+ }])?
    } => {{
        extern "C" fn __dome_cloomnik_method(vm: $crate::WrenVM) {
            $crate::__catch_panic_from_foreign(&vm, || <$($type)+>::$method(&vm));
        }
        unsafe {
            $ctx.register_fn(
                $module,
                concat!("static ", stringify!($class), ".[",
                    $crate::register_modules! { @underscore $param0 },
                    $(",", $crate::register_modules! { @underscore $params },)*
                "]"),
                __dome_cloomnik_method,
            )
        }
        .unwrap();
        $crate::register_modules! { @register_class_members
            ctx = [{ $ctx }]
            module = [{ $module }]
            class = [{ $class }]
            items = [{ $($rest)* }]
            type = [{ $($type)+ }]
            $(foreign_type = [{ $($foreign_type)+ }])?
        }
    }};
    // Instance subscript getter
    { @register_class_members
        ctx = [{ $ctx:expr }]
        module = [{ $module:literal }]
        class = [{ $class:ident }]
        items = [{
            foreign [$param0:ident $(, $params:ident)*] = $method:ident
            $($rest:tt)*
        }]
        type = [{ $($type:tt)+ }]
        $(foreign_type = [{ $($foreign_type:tt)+ }])?
    } => {{
        extern "C" fn __dome_cloomnik_method(vm: $crate::WrenVM) {
            $crate::__catch_panic_from_foreign(&vm, || {
                <$($type)+>::$method(
                    $(unsafe { vm.get_slot_foreign_unchecked::<$($foreign_type)+>(0) },)?
                    &vm,
                )
            });
        }
        unsafe {
            $ctx.register_fn(
                $module,
                concat!(stringify!($class), ".[",
                    $crate::register_modules! { @underscore $param0 },
                    $(",", $crate::register_modules! { @underscore $params },)*
                "]"),
                __dome_cloomnik_method,
            )
        }
        .unwrap();
        $crate::register_modules! { @register_class_members
            ctx = [{ $ctx }]
            module = [{ $module }]
            class = [{ $class }]
            items = [{ $($rest)* }]
            type = [{ $($type)+ }]
            $(foreign_type = [{ $($foreign_type)+ }])?
        }
    }};
    // Static subscript setter
    { @register_class_members
        ctx = [{ $ctx:expr }]
        module = [{ $module:literal }]
        class = [{ $class:ident }]
        items = [{
            foreign static [$param0:ident $(, $params:ident)*]=($value:ident) = $method:ident
            $($rest:tt)*
        }]
        type = [{ $($type:tt)+ }]
        $(foreign_type = [{ $($foreign_type:tt)+ }])?
    } => {{
        extern "C" fn __dome_cloomnik_method(vm: $crate::WrenVM) {
            $crate::__catch_panic_from_foreign(&vm, || <$($type)+>::$method(&vm));
        }
        unsafe {
            $ctx.register_fn(
                $module,
                concat!("static ", stringify!($class), ".[",
                    $crate::register_modules! { @underscore $param0 },
                    $(",", $crate::register_modules! { @underscore $params },)*
                "]=(_)"),
                __dome_cloomnik_method,
            )
        }
        .unwrap();
        $crate::register_modules! { @register_class_members
            ctx = [{ $ctx }]
            module = [{ $module }]
            class = [{ $class }]
            items = [{ $($rest)* }]
            type = [{ $($type)+ }]
            $(foreign_type = [{ $($foreign_type)+ }])?
        }
    }};
    // Instance subscript setter
    { @register_class_members
        ctx = [{ $ctx:expr }]
        module = [{ $module:literal }]
        class = [{ $class:ident }]
        items = [{
            foreign [$param0:ident $(, $params:ident)*]=($value:ident) = $method:ident
            $($rest:tt)*
        }]
        type = [{ $($type:tt)+ }]
        $(foreign_type = [{ $($foreign_type:tt)+ }])?
    } => {{
        extern "C" fn __dome_cloomnik_method(vm: $crate::WrenVM) {
            $crate::__catch_panic_from_foreign(&vm, || {
                <$($type)+>::$method(
                    $(unsafe { vm.get_slot_foreign_unchecked::<$($foreign_type)+>(0) },)?
                    &vm,
                )
            });
        }
        unsafe {
            $ctx.register_fn(
                $module,
                concat!(stringify!($class), ".[",
                    $crate::register_modules! { @underscore $param0 },
                    $(",", $crate::register_modules! { @underscore $params },)*
                "]=(_)"),
                __dome_cloomnik_method,
            )
        }
        .unwrap();
        $crate::register_modules! { @register_class_members
            ctx = [{ $ctx }]
            module = [{ $module }]
            class = [{ $class }]
            items = [{ $($rest)* }]
            type = [{ $($type)+ }]
            $(foreign_type = [{ $($foreign_type)+ }])?
        }
    }};
    // Non-foreign method (of any kind)
    { @register_class_members
        ctx = [{ $ctx:expr }]
        module = [{ $module:literal }]
        class = [{ $class:ident }]
        items = [{
            $method:literal
            $($rest:tt)*
        }]
        type = [{ $($type:tt)+ }]
        $(foreign_type = [{ $($foreign_type:tt)+ }])?
    } => {
        $crate::register_modules! { @register_class_members
            ctx = [{ $ctx }]
            module = [{ $module }]
            class = [{ $class }]
            items = [{ $($rest)* }]
            type = [{ $($type)+ }]
            $(foreign_type = [{ $($foreign_type)+ }])?
        }
    };
}
