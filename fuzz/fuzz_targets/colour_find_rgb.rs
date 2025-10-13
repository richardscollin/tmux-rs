#![no_main]

#[derive(arbitrary::Arbitrary, Debug)]
struct RgbInput {
    r: u8,
    g: u8,
    b: u8,
}

libfuzzer_sys::fuzz_target!(|input: RgbInput| {
    let RgbInput { r, g, b } = input;

    let current_result = tmux_rs_new::colour::colour_find_rgb(r, g, b);
    let old_result = tmux_rs_old::colour::colour_find_rgb(r, g, b);

    assert_eq!(
        current_result, old_result,
        "Regression detected!\nInput: r={r}, g={g}, b={b}\nCurrent: {current_result}\nOld: {old_result}",
    );
});
