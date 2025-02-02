use super::*;
unsafe extern "C" {
    pub fn screen_write_make_list(_: *mut screen);
    pub fn screen_write_free_list(_: *mut screen);
    pub fn screen_write_start_pane(_: *mut screen_write_ctx, _: *mut window_pane, _: *mut screen);
    pub fn screen_write_start(_: *mut screen_write_ctx, _: *mut screen);
    pub fn screen_write_start_callback(
        _: *mut screen_write_ctx,
        _: *mut screen,
        _: screen_write_init_ctx_cb,
        _: *mut c_void,
    );
    pub fn screen_write_stop(_: *mut screen_write_ctx);
    pub fn screen_write_reset(_: *mut screen_write_ctx);
    pub fn screen_write_strlen(_: *const c_char, ...) -> usize;
    pub fn screen_write_text(
        _: *mut screen_write_ctx,
        _: c_uint,
        _: c_uint,
        _: c_uint,
        _: c_int,
        _: *const grid_cell,
        _: *const c_char,
        ...
    ) -> c_int;
    pub fn screen_write_puts(_: *mut screen_write_ctx, _: *const grid_cell, _: *const c_char, ...);
    pub fn screen_write_nputs(_: *mut screen_write_ctx, _: isize, _: *const grid_cell, _: *const c_char, ...);
    pub fn screen_write_vnputs(
        _: *mut screen_write_ctx,
        _: isize,
        _: *const grid_cell,
        _: *const c_char,
        _: *mut VaList,
    );
    pub fn screen_write_putc(_: *mut screen_write_ctx, _: *const grid_cell, _: c_uchar);
    pub fn screen_write_fast_copy(_: *mut screen_write_ctx, _: *mut screen, _: c_uint, _: c_uint, _: c_uint, _: c_uint);
    pub fn screen_write_hline(
        _: *mut screen_write_ctx,
        _: c_uint,
        _: c_int,
        _: c_int,
        _: box_lines,
        _: *const grid_cell,
    );
    pub fn screen_write_vline(_: *mut screen_write_ctx, _: c_uint, _: c_int, _: c_int);
    pub fn screen_write_menu(
        _: *mut screen_write_ctx,
        _: *mut menu,
        _: c_int,
        _: box_lines,
        _: *const grid_cell,
        _: *const grid_cell,
        _: *const grid_cell,
    );
    pub fn screen_write_box(
        _: *mut screen_write_ctx,
        _: c_uint,
        _: c_uint,
        _: box_lines,
        _: *const grid_cell,
        _: *const c_char,
    );
    pub fn screen_write_preview(_: *mut screen_write_ctx, _: *mut screen, _: c_uint, _: c_uint);
    pub fn screen_write_backspace(_: *mut screen_write_ctx);
    pub fn screen_write_mode_set(_: *mut screen_write_ctx, _: c_int);
    pub fn screen_write_mode_clear(_: *mut screen_write_ctx, _: c_int);
    pub fn screen_write_cursorup(_: *mut screen_write_ctx, _: c_uint);
    pub fn screen_write_cursordown(_: *mut screen_write_ctx, _: c_uint);
    pub fn screen_write_cursorright(_: *mut screen_write_ctx, _: c_uint);
    pub fn screen_write_cursorleft(_: *mut screen_write_ctx, _: c_uint);
    pub fn screen_write_alignmenttest(_: *mut screen_write_ctx);
    pub fn screen_write_insertcharacter(_: *mut screen_write_ctx, _: c_uint, _: c_uint);
    pub fn screen_write_deletecharacter(_: *mut screen_write_ctx, _: c_uint, _: c_uint);
    pub fn screen_write_clearcharacter(_: *mut screen_write_ctx, _: c_uint, _: c_uint);
    pub fn screen_write_insertline(_: *mut screen_write_ctx, _: c_uint, _: c_uint);
    pub fn screen_write_deleteline(_: *mut screen_write_ctx, _: c_uint, _: c_uint);
    pub fn screen_write_clearline(_: *mut screen_write_ctx, _: c_uint);
    pub fn screen_write_clearendofline(_: *mut screen_write_ctx, _: c_uint);
    pub fn screen_write_clearstartofline(_: *mut screen_write_ctx, _: c_uint);
    pub fn screen_write_cursormove(_: *mut screen_write_ctx, _: c_int, _: c_int, _: c_int);
    pub fn screen_write_reverseindex(_: *mut screen_write_ctx, _: c_uint);
    pub fn screen_write_scrollregion(_: *mut screen_write_ctx, _: c_uint, _: c_uint);
    pub fn screen_write_linefeed(_: *mut screen_write_ctx, _: c_int, _: c_uint);
    pub fn screen_write_scrollup(_: *mut screen_write_ctx, _: c_uint, _: c_uint);
    pub fn screen_write_scrolldown(_: *mut screen_write_ctx, _: c_uint, _: c_uint);
    pub fn screen_write_carriagereturn(_: *mut screen_write_ctx);
    pub fn screen_write_clearendofscreen(_: *mut screen_write_ctx, _: c_uint);
    pub fn screen_write_clearstartofscreen(_: *mut screen_write_ctx, _: c_uint);
    pub fn screen_write_clearscreen(_: *mut screen_write_ctx, _: c_uint);
    pub fn screen_write_clearhistory(_: *mut screen_write_ctx);
    pub fn screen_write_fullredraw(_: *mut screen_write_ctx);
    pub fn screen_write_collect_end(_: *mut screen_write_ctx);
    pub fn screen_write_collect_add(_: *mut screen_write_ctx, _: *const grid_cell);
    pub fn screen_write_cell(_: *mut screen_write_ctx, _: *const grid_cell);
    pub fn screen_write_setselection(_: *mut screen_write_ctx, _: *const c_char, _: *mut c_uchar, _: c_uint);
    pub fn screen_write_rawstring(_: *mut screen_write_ctx, _: *mut c_uchar, _: c_uint, _: c_int);
    pub fn screen_write_alternateon(_: *mut screen_write_ctx, _: *mut grid_cell, _: c_int);
    pub fn screen_write_alternateoff(_: *mut screen_write_ctx, _: *mut grid_cell, _: c_int);
}
