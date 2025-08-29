# Changelog

## main

## 0.0.3

- Add sixel support
- Improved macOS installation process
- Change tmux-rs socket path to /tmp/tmux-rs-1000/
- Refactoring to use more Rust idioms and reduce unsafe code
- Fixed unnecessary cursor updates causing high CPU usage
- Fixed panic in debug build due to subtraction overflow in `grid_view_delete_cells`
- Fixed path display issues (due to incorrect strrchr implementation)
- Fixed mouse mode issues (selecting window by mouse, etc.)
- Fixed multiple bugs due to incorrect bitwise negation translation
- Fixed memory leak due to shadowing
- Fixed clear terminal on vim close (alternate screen check in `screen_write_alternateoff`)
- Fixed broken display-message command
- Fixed hex color parsing logic

## 0.0.2

- macOS and linux aarch64 supported added
- fixed numerous crashes
- fixed broken command line argument parsing
- Slightly improved panic log generation
- XDG\_CONFIG\_HOME now included in default config path
- added support to configure some variables at build time using environment variables:
  - TMUX\_VERSION
  - TMUX\_CONF
  - TMUX\_SOCK
  - TMUX\_TERM
  - TMUX\_LOCK\_CMD

## 0.0.1

initial release

