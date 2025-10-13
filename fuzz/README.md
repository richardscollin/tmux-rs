# Fuzzing tmux-rs

Commands should be run from the root of the tmux-rs repo.

List available fuzz targets:

    cargo fuzz list

Run a specific target:

    cargo fuzz run colour_find_rgb

Run with more cores:

    cargo fuzz run colour_find_rgb -- -jobs=8

Run for a specific duration:

    cargo fuzz run colour_find_rgb -- -max_total_time=60

