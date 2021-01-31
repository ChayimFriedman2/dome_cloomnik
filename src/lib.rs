mod audio;
mod dome;
mod wren;

#[no_mangle]
#[allow(non_snake_case)]
extern "C" fn PLUGIN_onInit(get_api: dome::GetApiFunction, ctx: dome::Context) -> dome::Result {
    dome::Result::Success
}

#[no_mangle]
#[allow(non_snake_case)]
extern "C" fn PLUGIN_preUpdate(ctx: dome::Context) -> dome::Result {
    dome::Result::Success
}

#[no_mangle]
#[allow(non_snake_case)]
extern "C" fn PLUGIN_postUpdate(ctx: dome::Context) -> dome::Result {
    dome::Result::Success
}

#[no_mangle]
#[allow(non_snake_case)]
extern "C" fn PLUGIN_preDraw(ctx: dome::Context) -> dome::Result {
    dome::Result::Success
}

#[no_mangle]
#[allow(non_snake_case)]
extern "C" fn PLUGIN_postDraw(ctx: dome::Context) -> dome::Result {
    dome::Result::Success
}

#[no_mangle]
#[allow(non_snake_case)]
extern "C" fn PLUGIN_onShutdown(ctx: dome::Context) -> dome::Result {
    dome::Result::Success
}
