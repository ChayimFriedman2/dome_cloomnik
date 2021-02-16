// Transformed directly from https://github.com/domeengine/dome/blob/ffc47ca273430c2da0b0479ab12f959e57d12ba9/examples/plugin/test.c

use dome_cloomnik::{register_modules, Context, WrenVM};

#[no_mangle]
#[allow(non_snake_case)]
extern "C" fn PLUGIN_onInit(get_api: *mut libc::c_void, ctx: *mut libc::c_void) -> libc::c_int {
    unsafe {
        dome_cloomnik::init_plugin(
            get_api,
            ctx,
            dome_cloomnik::Hooks {
                on_init: Some(on_init),
                pre_update: None,
                post_update: None,
                pre_draw: None,
                post_draw: None,
                on_shutdown: None,
            },
        )
    }
}

struct ExternalClass;
impl ExternalClass {
    fn init(_vm: &WrenVM) -> Self {
        ExternalClass
    }

    fn alert(&mut self, vm: &WrenVM) {
        let mut text = vm.get_slot_string(1).expect("Invalid text.");
        text += "\n";
        vm.get_context().log(&text);
    }
}

fn on_init(ctx: Context) -> Result<(), ()> {
    ctx.log("Initialising external module\n");

    register_modules! {
        ctx,
        module "external" {
            foreign class ExternalClass = init of ExternalClass {
                "construct init() {}"
                foreign alert(text) = alert
            }
        }
    };

    Ok(())
}
